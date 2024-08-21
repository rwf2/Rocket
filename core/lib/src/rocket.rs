use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;
use std::any::Any;
use std::future::Future;
use std::panic::Location;

use either::Either;
use figment::{Figment, Provider};
use futures::TryFutureExt;

use crate::shutdown::{Stages, Shutdown};
use crate::trace::{Trace, TraceAll};
use crate::{sentinel, shield::Shield, Catcher, Config, Route};
use crate::listener::{Bind, DefaultListener, Endpoint, Listener};
use crate::router::Router;
use crate::fairing::{Fairing, Fairings};
use crate::phase::{Phase, Build, Building, Ignite, Igniting, Orbit, Orbiting};
use crate::phase::{Stateful, StateRef, StateRefMut, State};
use crate::http::uri::Origin;
use crate::http::ext::IntoOwned;
use crate::error::{Error, ErrorKind};

/// The application server itself.
///
/// # Phases
///
/// A `Rocket` instance represents a web server and its state. It progresses
/// through three statically-enforced phases: build, ignite, orbit.
///
/// * **Build**: _application and server configuration_
///
///   This phase enables:
///
///     * setting configuration options
///     * mounting/registering routes/catchers
///     * managing state
///     * attaching fairings
///
///   This is the _only_ phase in which an instance can be modified. To finalize
///   changes, an instance is ignited via [`Rocket::ignite()`], progressing it
///   into the _ignite_ phase, or directly launched into orbit with
///   [`Rocket::launch()`] which progress the instance through ignite into
///   orbit.
///
/// * **Ignite**: _verification and finalization of configuration_
///
///   An instance in the [`Ignite`] phase is in its final configuration,
///   available via [`Rocket::config()`]. Barring user-supplied interior
///   mutation, application state is guaranteed to remain unchanged beyond this
///   point. An instance in the ignite phase can be launched into orbit to serve
///   requests via [`Rocket::launch()`].
///
/// * **Orbit**: _a running web server_
///
///   An instance in the [`Orbit`] phase represents a _running_ application,
///   actively serving requests.
///
/// # Launching
///
/// To launch a `Rocket` application, the suggested approach is to return an
/// instance of `Rocket<Build>` from a function named `rocket` marked with the
/// [`#[launch]`](crate::launch) attribute:
///
///   ```rust,no_run
///   # use rocket::launch;
///   #[launch]
///   fn rocket() -> _ {
///       rocket::build()
///   }
///   ```
///
/// This generates a `main` function with an `async` runtime that runs the
/// returned `Rocket` instance.
///
/// * **Manual Launching**
///
///   To launch an instance of `Rocket`, it _must_ progress through all three
///   phases. To progress into the ignite or launch phases, a tokio `async`
///   runtime is required. The [`#[main]`](crate::main) attribute initializes a
///   Rocket-specific tokio runtime and runs the attributed `async fn` inside of
///   it:
///
///   ```rust,no_run
///   #[rocket::main]
///   async fn main() -> Result<(), rocket::Error> {
///       let _rocket = rocket::build()
///           .ignite().await?
///           .launch().await?;
///
///       Ok(())
///   }
///   ```
///
///   Note that [`Rocket::launch()`] automatically progresses an instance of
///   `Rocket` from any phase into orbit:
///
///   ```rust,no_run
///   #[rocket::main]
///   async fn main() -> Result<(), rocket::Error> {
///       let _rocket = rocket::build().launch().await?;
///       Ok(())
///   }
///   ```
///
///   For extreme and rare cases in which [`#[main]`](crate::main) imposes
///   obstinate restrictions, use [`rocket::execute()`](crate::execute()) to
///   execute Rocket's `launch()` future.
///
/// * **Automatic Launching**
///
///   Manually progressing an instance of Rocket though its phases is only
///   necessary when either an instance's finalized state is to be inspected (in
///   the _ignite_ phase) or the instance is expected to deorbit due to
///   [`Rocket::shutdown()`]. In the more common case when neither is required,
///   the [`#[launch]`](crate::launch) attribute can be used. When applied to a
///   function that returns a `Rocket<Build>`, it automatically initializes an
///   `async` runtime and launches the function's returned instance:
///
///   ```rust,no_run
///   # use rocket::launch;
///   use rocket::{Rocket, Build};
///
///   #[launch]
///   fn rocket() -> Rocket<Build> {
///       rocket::build()
///   }
///   ```
///
///   To avoid needing to import _any_ items in the common case, the `launch`
///   attribute will infer a return type written as `_` as `Rocket<Build>`:
///
///   ```rust,no_run
///   # use rocket::launch;
///   #[launch]
///   fn rocket() -> _ {
///       rocket::build()
///   }
///   ```
pub struct Rocket<P: Phase>(pub(crate) P::State);

impl Rocket<Build> {
    /// Create a new `Rocket` application using the default configuration
    /// provider, [`Config::figment()`].
    ///
    /// This method is typically called through the
    /// [`rocket::build()`](crate::build) alias.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::launch;
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    /// }
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn build() -> Self {
        Rocket::custom(Config::figment())
    }

    /// Creates a new `Rocket` application using the supplied configuration
    /// provider.
    ///
    /// This method is typically called through the
    /// [`rocket::custom()`](crate::custom()) alias.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::launch;
    /// use rocket::figment::{Figment, providers::{Toml, Env, Format}};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let figment = Figment::from(rocket::Config::default())
    ///         .merge(Toml::file("MyApp.toml").nested())
    ///         .merge(Env::prefixed("MY_APP_").global());
    ///
    ///     rocket::custom(figment)
    /// }
    /// ```
    #[must_use]
    pub fn custom<T: Provider>(provider: T) -> Self {
        Rocket::<Build>(Building::default())
            .reconfigure(provider)
            .attach(Shield::default())
    }

    /// Overrides the current configuration provider with `provider`.
    ///
    /// The default provider, or a provider previously set with
    /// [`Rocket::custom()`] or [`Rocket::reconfigure()`], is overridden by
    /// `provider`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Ident};
    /// # use std::net::Ipv4Addr;
    /// # use std::path::{Path, PathBuf};
    /// # type Result = std::result::Result<(), rocket::Error>;
    ///
    /// let config = Config {
    ///     ident: Ident::try_new("MyServer").expect("valid ident"),
    ///     temp_dir: "/tmp/config-example".into(),
    ///     ..Config::debug_default()
    /// };
    ///
    /// # let _: Result = rocket::async_test(async move {
    /// let rocket = rocket::custom(&config).ignite().await?;
    /// assert_eq!(rocket.config().ident.as_str(), Some("MyServer"));
    /// assert_eq!(rocket.config().temp_dir.relative(), Path::new("/tmp/config-example"));
    ///
    /// // Create a new figment which modifies _some_ keys the existing figment:
    /// let figment = rocket.figment().clone()
    ///     .merge((Config::IDENT, "Example"));
    ///
    /// let rocket = rocket::custom(&config)
    ///     .reconfigure(figment)
    ///     .ignite().await?;
    ///
    /// assert_eq!(rocket.config().ident.as_str(), Some("Example"));
    /// assert_eq!(rocket.config().temp_dir.relative(), Path::new("/tmp/config-example"));
    /// # Ok(())
    /// # });
    /// ```
    #[must_use]
    pub fn reconfigure<T: Provider>(mut self, provider: T) -> Self {
        // We initialize the logger here so that logging from fairings and so on
        // are visible; we use the final config to set a max log-level in ignite
        self.figment = Figment::from(provider);
        crate::trace::init(Config::try_from(&self.figment).ok().as_ref());
        span_trace!("reconfigure" => self.figment().trace_trace());

        self
    }

    #[track_caller]
    fn load<'a, B, T, F, M>(mut self, kind: &str, base: B, items: Vec<T>, m: M, f: F) -> Self
        where B: TryInto<Origin<'a>> + Clone + fmt::Display,
              B::Error: fmt::Display,
              M: Fn(&Origin<'a>, T) -> T,
              F: Fn(&mut Self, T),
              T: Clone + Trace,
    {
        let mut base = match base.clone().try_into() {
            Ok(origin) => origin.into_owned(),
            Err(e) => {
                error!(%base, location = %Location::caller(), "invalid {kind} base uri: {e}");
                panic!("aborting due to {} base error", kind);
            }
        };

        if base.query().is_some() {
            warn!(%base, location = %Location::caller(), "query in {kind} base is ignored");
            base.clear_query();
        }

        for unmounted_item in items {
            f(&mut self, m(&base, unmounted_item.clone()))
        }

        self
    }

    /// Mounts all of the `routes` at the given `base` mount point.
    ///
    /// A route _mounted_ at `base` has an effective URI of `base/route`, where
    /// `route` is the route URI. In other words, `base` is added as a prefix to
    /// the route's URI. The URI resulting from joining the `base` URI and the
    /// route URI is called the route's _effective URI_, as this is the URI used
    /// for request matching during routing.
    ///
    /// A `base` URI is not allowed to have a query part. If a `base` _does_
    /// have a query part, it is ignored when producing the effective URI.
    ///
    /// A `base` may have an optional trailing slash. A route with a URI path of
    /// `/` (and any optional query) mounted at a `base` has an effective URI
    /// equal to the `base` (plus any optional query). That is, if the base has
    /// a trailing slash, the effective URI path has a trailing slash, and
    /// otherwise it does not. Routes with URI paths other than `/` are not
    /// effected by trailing slashes in their corresponding mount point.
    ///
    /// As concrete examples, consider the following table:
    ///
    /// | mount point | route URI | effective URI |
    /// |-------------|-----------|---------------|
    /// | `/`         | `/foo`    | `/foo`        |
    /// | `/`         | `/foo/`   | `/foo/`       |
    /// | `/foo`      | `/`       | `/foo`        |
    /// | `/foo`      | `/?bar`   | `/foo?bar`    |
    /// | `/foo`      | `/bar`    | `/foo/bar`    |
    /// | `/foo`      | `/bar/`   | `/foo/bar/`   |
    /// | `/foo/`     | `/`       | `/foo/`       |
    /// | `/foo/`     | `/bar`    | `/foo/bar`    |
    /// | `/foo/`     | `/?bar`   | `/foo/?bar`   |
    /// | `/foo/bar`  | `/`       | `/foo/bar`    |
    /// | `/foo/bar/` | `/`       | `/foo/bar/`   |
    /// | `/foo/?bar` | `/`       | `/foo/`       |
    /// | `/foo/?bar` | `/baz`    | `/foo/baz`    |
    /// | `/foo/?bar` | `/baz/`   | `/foo/baz/`   |
    ///
    /// # Panics
    ///
    /// Panics if either:
    ///
    ///   * the `base` mount point is not a valid origin URI without dynamic
    ///     parameters
    ///
    ///   * any route URI is not a valid origin URI. (**Note:** _This kind of
    ///     panic is guaranteed not to occur if the routes were generated using
    ///     Rocket's code generation._)
    ///
    /// # Examples
    ///
    /// Use the `routes!` macro to mount routes created using the code
    /// generation facilities. Requests to both `/world` and `/hello/world` URI
    /// will be dispatched to the `hi` route.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// #
    /// #[get("/world")]
    /// fn hi() -> &'static str {
    ///     "Hello!"
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/", routes![hi])
    ///         .mount("/hello", routes![hi])
    /// }
    /// ```
    ///
    /// Manually create a route named `hi` at path `"/world"` mounted at base
    /// `"/hello"`. Requests to the `/hello/world` URI will be dispatched to the
    /// `hi` route.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::{Request, Route, Data, route};
    /// use rocket::http::Method;
    ///
    /// fn hi<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
    ///     route::Outcome::from(req, "Hello!").pin()
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let hi_route = Route::new(Method::Get, "/world", hi);
    ///     rocket::build().mount("/hello", vec![hi_route])
    /// }
    /// ```
    #[must_use]
    #[track_caller]
    pub fn mount<'a, B, R>(self, base: B, routes: R) -> Self
        where B: TryInto<Origin<'a>> + Clone + fmt::Display,
              B::Error: fmt::Display,
              R: Into<Vec<Route>>
    {
        self.load("route", base, routes.into(),
            |base, route| route.rebase(base.clone()),
            |r, route| r.0.routes.push(route))
    }

    /// Registers all of the catchers in the supplied vector, scoped to `base`.
    ///
    /// # Panics
    ///
    /// Panics if `base` is not a valid static path: a valid origin URI without
    /// dynamic parameters.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Request;
    ///
    /// #[catch(500)]
    /// fn internal_error() -> &'static str {
    ///     "Whoops! Looks like we messed up."
    /// }
    ///
    /// #[catch(404)]
    /// fn not_found(req: &Request) -> String {
    ///     format!("I couldn't find '{}'. Try something else?", req.uri())
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().register("/", catchers![internal_error, not_found])
    /// }
    /// ```
    #[must_use]
    pub fn register<'a, B, C>(self, base: B, catchers: C) -> Self
        where B: TryInto<Origin<'a>> + Clone + fmt::Display,
              B::Error: fmt::Display,
              C: Into<Vec<Catcher>>
    {
        self.load("catcher", base, catchers.into(),
            |base, catcher| catcher.rebase(base.clone()),
            |r, catcher| r.0.catchers.push(catcher))
    }

    /// Add `state` to the state managed by this instance of Rocket.
    ///
    /// This method can be called any number of times as long as each call
    /// refers to a different `T`.
    ///
    /// Managed state can be retrieved by any request handler via the
    /// [`State`](crate::State) request guard. In particular, if a value of type `T`
    /// is managed by Rocket, adding `State<T>` to the list of arguments in a
    /// request handler instructs Rocket to retrieve the managed value.
    ///
    /// # Panics
    ///
    /// Panics if state of type `T` is already being managed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::State;
    ///
    /// struct MyInt(isize);
    /// struct MyString(String);
    ///
    /// #[get("/int")]
    /// fn int(state: &State<MyInt>) -> String {
    ///     format!("The stateful int is: {}", state.0)
    /// }
    ///
    /// #[get("/string")]
    /// fn string(state: &State<MyString>) -> &str {
    ///     &state.0
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .manage(MyInt(10))
    ///         .manage(MyString("Hello, managed state!".to_string()))
    ///         .mount("/", routes![int, string])
    /// }
    /// ```
    #[must_use]
    pub fn manage<T>(self, state: T) -> Self
        where T: Send + Sync + 'static
    {
        let type_name = std::any::type_name::<T>();
        if !self.state.set(state) {
            error!("state for type '{}' is already being managed", type_name);
            panic!("aborting due to duplicated managed state");
        }

        self
    }

    /// Attaches a fairing to this instance of Rocket. No fairings are eagerly
    /// executed; fairings are executed at their appropriate time.
    ///
    /// If the attached fairing is a [singleton] and a fairing of the same type
    /// has already been attached, this fairing replaces it. Otherwise the
    /// fairing gets attached without replacing any existing fairing.
    ///
    /// [singleton]: crate::fairing::Fairing#singletons
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .attach(AdHoc::on_liftoff("Liftoff Message", |_| Box::pin(async {
    ///             println!("We have liftoff!");
    ///         })))
    /// }
    /// ```
    #[must_use]
    pub fn attach<F: Fairing>(mut self, fairing: F) -> Self {
        self.fairings.add(Box::new(fairing));
        self
    }

    /// Returns a `Future` that transitions this instance of `Rocket` into the
    /// _ignite_ phase.
    ///
    /// When `await`ed, the future runs all _ignite_ fairings in serial,
    /// [attach](Rocket::attach()) order, and verifies that `self` represents a
    /// valid instance of `Rocket` ready for launch. This means that:
    ///
    ///   * All ignite fairings succeeded.
    ///   * A valid [`Config`] was extracted from [`Rocket::figment()`].
    ///   * If `secrets` are enabled, the extracted `Config` contains a safe
    ///     secret key.
    ///   * There are no [`Route#collisions`] or [`Catcher#collisions`]
    ///     collisions.
    ///   * No [`Sentinel`](crate::Sentinel) triggered an abort.
    ///
    /// If any of these conditions fail to be met, a respective [`Error`] is
    /// returned.
    ///
    /// [configured]: Rocket::figment()
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// #[rocket::main]
    /// async fn main() -> Result<(), rocket::Error> {
    ///     let rocket = rocket::build()
    ///         # .reconfigure(rocket::Config::debug_default())
    ///         .attach(AdHoc::on_ignite("Manage State", |rocket| async move {
    ///             rocket.manage(String::from("managed string"))
    ///         }));
    ///
    ///     // No fairings are run until ignition occurs.
    ///     assert!(rocket.state::<String>().is_none());
    ///
    ///     let rocket = rocket.ignite().await?;
    ///     assert_eq!(rocket.state::<String>().unwrap(), "managed string");
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn ignite(mut self) -> Result<Rocket<Ignite>, Error> {
        self = Fairings::handle_ignite(self).await;
        self.fairings.audit().map_err(|f| ErrorKind::FailedFairings(f.to_vec()))?;

        // Extract the configuration; initialize default trace subscriber.
        #[allow(unused_mut)]
        let mut config = Config::try_from(&self.figment).map_err(ErrorKind::Config)?;
        crate::trace::init(&config);

        // Check for safely configured secrets.
        #[cfg(feature = "secrets")]
        if !config.secret_key.is_provided() {
            if config.profile != Config::DEBUG_PROFILE {
                return Err(Error::new(ErrorKind::InsecureSecretKey(config.profile.clone())));
            }

            if config.secret_key.is_zero() {
                config.secret_key = crate::config::SecretKey::generate()
                    .unwrap_or_else(crate::config::SecretKey::zero);
            }
        }

        // Initialize the router; check for collisions.
        let mut router = Router::new();
        self.routes.clone().into_iter().for_each(|r| router.add_route(r));
        self.catchers.clone().into_iter().for_each(|c| router.add_catcher(c));
        router.finalize().map_err(|(r, c)| ErrorKind::Collisions { routes: r, catchers: c, })?;

        // Finally, freeze managed state for faster access later.
        self.state.freeze();

        // Log everything we know: config, routes, catchers, fairings.
        // TODO: Store/print managed state type names?
        let fairings = self.fairings.unique_set();
        span_info!("config", profile = %self.figment().profile() => {
            config.trace_info();
            self.figment().trace_debug();
        });

        span_info!("routes", count = self.routes.len() => self.routes().trace_all_info());
        span_info!("catchers", count = self.catchers.len() => self.catchers().trace_all_info());
        span_info!("fairings", count = fairings.len() => fairings.trace_all_info());

        // Ignite the rocket.
        let rocket: Rocket<Ignite> = Rocket(Igniting {
            shutdown: Stages::new(),
            figment: self.0.figment,
            fairings: self.0.fairings,
            state: self.0.state,
            router, config,
        });

        // Query the sentinels, abort if requested.
        let sentinels = rocket.routes().flat_map(|r| r.sentinels.iter());
        sentinel::query(sentinels, &rocket).map_err(ErrorKind::SentinelAborts)?;

        Ok(rocket)
    }
}

impl Rocket<Ignite> {
    /// Returns the finalized, active configuration. This is guaranteed to
    /// remain stable through ignition and into orbit.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// #[rocket::main]
    /// async fn main() -> Result<(), rocket::Error> {
    ///     let rocket = rocket::build().ignite().await?;
    ///     let config = rocket.config();
    ///     Ok(())
    /// }
    /// ```
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a handle which can be used to trigger a shutdown and detect a
    /// triggered shutdown.
    ///
    /// A completed graceful shutdown resolves the future returned by
    /// [`Rocket::launch()`]. If [`Shutdown::notify()`] is called _before_ an
    /// instance is launched, it will be immediately shutdown after liftoff. See
    /// [`Shutdown`] and [`ShutdownConfig`](crate::config::ShutdownConfig) for
    /// details on graceful shutdown.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use std::time::Duration;
    /// use rocket::tokio::{self, time};
    ///
    /// #[rocket::main]
    /// async fn main() -> Result<(), rocket::Error> {
    ///     let rocket = rocket::build().ignite().await?;
    ///
    ///     let shutdown = rocket.shutdown();
    ///     tokio::spawn(async move {
    ///         time::sleep(time::Duration::from_secs(5)).await;
    ///         shutdown.notify();
    ///     });
    ///
    ///     // The `launch()` future resolves after ~5 seconds.
    ///     let result = rocket.launch().await;
    ///     assert!(result.is_ok());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown.start.clone()
    }

    pub(crate) fn into_orbit(self, endpoints: Vec<Endpoint>) -> Rocket<Orbit> {
        Rocket(Orbiting {
            endpoints,
            router: self.0.router,
            fairings: self.0.fairings,
            figment: self.0.figment,
            config: self.0.config,
            state: self.0.state,
            shutdown: self.0.shutdown,
        })
    }

    async fn _local_launch(self, endpoint: Endpoint) -> Rocket<Orbit> {
        let rocket = self.into_orbit(vec![endpoint]);
        Rocket::liftoff(&rocket).await;
        rocket
    }

    async fn _launch<L: Listener + 'static>(self, listener: L) -> Result<Rocket<Ignite>, Error> {
        let rocket = self.listen_and_serve(listener, |rocket| async move {
            let rocket = Arc::new(rocket);

            rocket.shutdown.spawn_listener(&rocket.config.shutdown);
            if let Err(e) = tokio::spawn(Rocket::liftoff(rocket.clone())).await {
                let rocket = rocket.try_wait_shutdown().await.map(Box::new);
                return Err(ErrorKind::Liftoff(rocket, e).into());
            }

            Ok(rocket)
        }).await?;

        Ok(rocket.try_wait_shutdown().await.map_err(ErrorKind::Shutdown)?)
    }
}

impl Rocket<Orbit> {
    /// Rocket wraps all connections in a `CancellableIo` struct, an internal
    /// structure that gracefully closes I/O when it receives a signal. That
    /// signal is the `shutdown` future. When the future resolves,
    /// `CancellableIo` begins to terminate in grace, mercy, and finally force
    /// close phases. Since all connections are wrapped in `CancellableIo`, this
    /// eventually ends all I/O.
    ///
    /// At that point, unless a user spawned an infinite, stand-alone task that
    /// isn't monitoring `Shutdown`, all tasks should resolve. This means that
    /// all instances of the shared `Arc<Rocket>` are dropped and we can return
    /// the owned instance of `Rocket`.
    ///
    /// Unfortunately, the Hyper `server` future resolves as soon as it has
    /// finished processing requests without respect for ongoing responses. That
    /// is, `server` resolves even when there are running tasks that are
    /// generating a response. So, `server` resolving implies little to nothing
    /// about the state of connections. As a result, we depend on the timing of
    /// grace + mercy + some buffer to determine when all connections should be
    /// closed, thus all tasks should be complete, thus all references to
    /// `Arc<Rocket>` should be dropped and we can get back a unique reference.
    async fn try_wait_shutdown(self: Arc<Self>) -> Result<Rocket<Ignite>, Arc<Self>> {
        info!("Shutting down. Waiting for shutdown fairings and pending I/O...");
        tokio::spawn({
            let rocket = self.clone();
            async move { rocket.fairings.handle_shutdown(&rocket).await }
        });

        let config = &self.config.shutdown;
        let wait = Duration::from_micros(250);
        for period in [wait, config.grace(), wait, config.mercy(), wait * 4] {
            if Arc::strong_count(&self) == 1 { break }
            tokio::time::sleep(period).await;
        }

        match Arc::try_unwrap(self) {
            Ok(rocket) => {
                info!("Graceful shutdown completed successfully.");
                Ok(rocket.deorbit())
            }
            Err(rocket) => {
                warn!("Shutdown failed: outstanding background I/O.");
                Err(rocket)
            }
        }
    }

    pub(crate) fn deorbit(self) -> Rocket<Ignite> {
        Rocket(Igniting {
            router: self.0.router,
            fairings: self.0.fairings,
            figment: self.0.figment,
            config: self.0.config,
            state: self.0.state,
            shutdown: self.0.shutdown,
        })
    }

    pub(crate) async fn liftoff<R: Deref<Target = Self>>(rocket: R) {
        let rocket = rocket.deref();
        rocket.fairings.handle_liftoff(rocket).await;

        if !crate::running_within_rocket_async_rt().await {
            warn!(
                "Rocket is executing inside of a custom runtime.\n\
                Rocket's runtime is enabled via `#[rocket::main]` or `#[launch]`\n\
                Forced shutdown is disabled. Runtime settings may be suboptimal."
            );
        }

        tracing::info!(name: "liftoff", endpoint = %rocket.endpoints[0]);
    }

    /// Returns the finalized, active configuration. This is guaranteed to
    /// remain stable after [`Rocket::ignite()`], through ignition and into
    /// orbit.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .attach(AdHoc::on_liftoff("Config", |rocket| Box::pin(async move {
    ///             println!("Rocket launch config: {:?}", rocket.config());
    ///         })))
    /// }
    /// ```
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &Endpoint> {
        self.endpoints.iter()
    }

    /// Returns a handle which can be used to trigger a shutdown and detect a
    /// triggered shutdown.
    ///
    /// A completed graceful shutdown resolves the future returned by
    /// [`Rocket::launch()`]. See [`Shutdown`] and
    /// [`ShutdownConfig`](crate::config::ShutdownConfig) for details on
    /// graceful shutdown.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::tokio::{self, time};
    /// use rocket::fairing::AdHoc;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .attach(AdHoc::on_liftoff("Shutdown", |rocket| Box::pin(async move {
    ///             let shutdown = rocket.shutdown();
    ///             tokio::spawn(async move {
    ///                 time::sleep(time::Duration::from_secs(5)).await;
    ///                 shutdown.notify();
    ///             });
    ///         })))
    /// }
    /// ```
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown.start.clone()
    }
}

impl<P: Phase> Rocket<P> {
    /// Returns an iterator over all of the routes mounted on this instance of
    /// Rocket. The order is unspecified.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::*;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[get("/hello")]
    /// fn hello() -> &'static str {
    ///     "Hello, world!"
    /// }
    ///
    /// let rocket = rocket::build()
    ///     .mount("/", routes![hello])
    ///     .mount("/hi", routes![hello]);
    ///
    /// assert_eq!(rocket.routes().count(), 2);
    /// assert!(rocket.routes().any(|r| r.uri == "/hello"));
    /// assert!(rocket.routes().any(|r| r.uri == "/hi/hello"));
    /// ```
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        match self.0.as_ref() {
            StateRef::Build(p) => Either::Left(p.routes.iter()),
            StateRef::Ignite(p) => Either::Right(p.router.routes()),
            StateRef::Orbit(p) => Either::Right(p.router.routes()),
        }
    }

    /// Returns an iterator over all of the catchers registered on this instance
    /// of Rocket. The order is unspecified.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::*;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[catch(404)] fn not_found() -> &'static str { "Nothing here, sorry!" }
    /// #[catch(500)] fn just_500() -> &'static str { "Whoops!?" }
    /// #[catch(default)] fn some_default() -> &'static str { "Everything else." }
    ///
    /// let rocket = rocket::build()
    ///     .register("/foo", catchers![not_found])
    ///     .register("/", catchers![just_500, some_default]);
    ///
    /// assert_eq!(rocket.catchers().count(), 3);
    /// assert!(rocket.catchers().any(|c| c.code == Some(404) && c.base() == "/foo"));
    /// assert!(rocket.catchers().any(|c| c.code == Some(500) && c.base() == "/"));
    /// assert!(rocket.catchers().any(|c| c.code == None && c.base() == "/"));
    /// ```
    pub fn catchers(&self) -> impl Iterator<Item = &Catcher> {
        match self.0.as_ref() {
            StateRef::Build(p) => Either::Left(p.catchers.iter()),
            StateRef::Ignite(p) => Either::Right(p.router.catchers()),
            StateRef::Orbit(p) => Either::Right(p.router.catchers()),
        }
    }

    /// Returns `Some` of the managed state value for the type `T` if it is
    /// being managed by `self`. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[derive(PartialEq, Debug)]
    /// struct MyState(&'static str);
    ///
    /// let rocket = rocket::build().manage(MyState("hello!"));
    /// assert_eq!(rocket.state::<MyState>().unwrap(), &MyState("hello!"));
    /// ```
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        match self.0.as_ref() {
            StateRef::Build(p) => p.state.try_get(),
            StateRef::Ignite(p) => p.state.try_get(),
            StateRef::Orbit(p) => p.state.try_get(),
        }
    }

    /// Returns a reference to the first fairing of type `F` if it is attached.
    /// Otherwise, returns `None`.
    ///
    /// To retrieve a _mutable_ reference to fairing `F`, use
    /// [`Rocket::fairing_mut()`] instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Rocket, Request, Data, Response, Build, Orbit};
    /// # use rocket::fairing::{self, Fairing, Info, Kind};
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MyFairing {
    /// #     fn info(&self) -> Info {
    /// #       Info { name: "", kind: Kind::Ignite  }
    /// #     }
    /// # }
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MySingletonFairing {
    /// #     fn info(&self) -> Info {
    /// #       Info { name: "", kind: Kind::Ignite | Kind::Singleton }
    /// #     }
    /// # }
    /// // A regular, non-singleton fairing.
    /// struct MyFairing(&'static str);
    ///
    /// // A singleton fairing.
    /// struct MySingletonFairing(&'static str);
    ///
    /// // fairing is not attached, returns `None`
    /// let rocket = rocket::build();
    /// assert!(rocket.fairing::<MyFairing>().is_none());
    /// assert!(rocket.fairing::<MySingletonFairing>().is_none());
    ///
    /// // attach fairing, now returns `Some`
    /// let rocket = rocket.attach(MyFairing("some state"));
    /// assert!(rocket.fairing::<MyFairing>().is_some());
    /// assert_eq!(rocket.fairing::<MyFairing>().unwrap().0, "some state");
    ///
    /// // it returns the first fairing of a given type only
    /// let rocket = rocket.attach(MyFairing("other state"));
    /// assert_eq!(rocket.fairing::<MyFairing>().unwrap().0, "some state");
    ///
    /// // attach fairing, now returns `Some`
    /// let rocket = rocket.attach(MySingletonFairing("first"));
    /// assert_eq!(rocket.fairing::<MySingletonFairing>().unwrap().0, "first");
    ///
    /// // recall that new singletons replace existing attached singletons
    /// let rocket = rocket.attach(MySingletonFairing("second"));
    /// assert_eq!(rocket.fairing::<MySingletonFairing>().unwrap().0, "second");
    /// ```
    pub fn fairing<F: Fairing>(&self) -> Option<&F> {
        match self.0.as_ref() {
            StateRef::Build(p) => p.fairings.filter::<F>().next(),
            StateRef::Ignite(p) => p.fairings.filter::<F>().next(),
            StateRef::Orbit(p) => p.fairings.filter::<F>().next(),
        }
    }

    /// Returns an iterator over all attached fairings of type `F`, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Rocket, Request, Data, Response, Build, Orbit};
    /// # use rocket::fairing::{self, Fairing, Info, Kind};
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MyFairing {
    /// #     fn info(&self) -> Info {
    /// #         Info { name: "", kind: Kind::Ignite  }
    /// #     }
    /// # }
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MySingletonFairing {
    /// #     fn info(&self) -> Info {
    /// #         Info { name: "", kind: Kind::Ignite | Kind::Singleton }
    /// #     }
    /// # }
    /// // A regular, non-singleton fairing.
    /// struct MyFairing(&'static str);
    ///
    /// // A singleton fairing.
    /// struct MySingletonFairing(&'static str);
    ///
    /// let rocket = rocket::build();
    /// assert_eq!(rocket.fairings::<MyFairing>().count(), 0);
    /// assert_eq!(rocket.fairings::<MySingletonFairing>().count(), 0);
    ///
    /// let rocket = rocket.attach(MyFairing("some state"))
    ///     .attach(MySingletonFairing("first"))
    ///     .attach(MySingletonFairing("second"))
    ///     .attach(MyFairing("other state"))
    ///     .attach(MySingletonFairing("third"));
    ///
    /// let my_fairings: Vec<_> = rocket.fairings::<MyFairing>().collect();
    /// assert_eq!(my_fairings.len(), 2);
    /// assert_eq!(my_fairings[0].0, "some state");
    /// assert_eq!(my_fairings[1].0, "other state");
    ///
    /// let my_singleton: Vec<_> = rocket.fairings::<MySingletonFairing>().collect();
    /// assert_eq!(my_singleton.len(), 1);
    /// assert_eq!(my_singleton[0].0, "third");
    /// ```
    pub fn fairings<F: Fairing>(&self) -> impl Iterator<Item = &F> {
        match self.0.as_ref() {
            StateRef::Build(p) => Either::Left(p.fairings.filter::<F>()),
            StateRef::Ignite(p) => Either::Right(p.fairings.filter::<F>()),
            StateRef::Orbit(p) => Either::Right(p.fairings.filter::<F>()),
        }
    }

    /// Returns a mutable reference to the first fairing of type `F` if it is
    /// attached. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Rocket, Request, Data, Response, Build, Orbit};
    /// # use rocket::fairing::{self, Fairing, Info, Kind};
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MyFairing {
    /// #     fn info(&self) -> Info {
    /// #       Info { name: "", kind: Kind::Ignite  }
    /// #     }
    /// # }
    /// // A regular, non-singleton fairing.
    /// struct MyFairing(&'static str);
    ///
    /// // fairing is not attached, returns `None`
    /// let mut rocket = rocket::build();
    /// assert!(rocket.fairing_mut::<MyFairing>().is_none());
    ///
    /// // attach fairing, now returns `Some`
    /// let mut rocket = rocket.attach(MyFairing("some state"));
    /// assert!(rocket.fairing_mut::<MyFairing>().is_some());
    /// assert_eq!(rocket.fairing_mut::<MyFairing>().unwrap().0, "some state");
    ///
    /// // we can modify the fairing
    /// rocket.fairing_mut::<MyFairing>().unwrap().0 = "other state";
    /// assert_eq!(rocket.fairing_mut::<MyFairing>().unwrap().0, "other state");
    ///
    /// // it returns the first fairing of a given type only
    /// let mut rocket = rocket.attach(MyFairing("yet more state"));
    /// assert_eq!(rocket.fairing_mut::<MyFairing>().unwrap().0, "other state");
    /// ```
    pub fn fairing_mut<F: Fairing>(&mut self) -> Option<&mut F> {
        match self.0.as_mut() {
            StateRefMut::Build(p) => p.fairings.filter_mut::<F>().next(),
            StateRefMut::Ignite(p) => p.fairings.filter_mut::<F>().next(),
            StateRefMut::Orbit(p) => p.fairings.filter_mut::<F>().next(),
        }
    }

    /// Returns an iterator of mutable references to all attached fairings of
    /// type `F`, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Rocket, Request, Data, Response, Build, Orbit};
    /// # use rocket::fairing::{self, Fairing, Info, Kind};
    /// #
    /// # #[rocket::async_trait]
    /// # impl Fairing for MyFairing {
    /// #     fn info(&self) -> Info {
    /// #         Info { name: "", kind: Kind::Ignite  }
    /// #     }
    /// # }
    /// // A regular, non-singleton fairing.
    /// struct MyFairing(&'static str);
    ///
    /// let mut rocket = rocket::build()
    ///     .attach(MyFairing("some state"))
    ///     .attach(MyFairing("other state"))
    ///     .attach(MyFairing("yet more state"));
    ///
    /// let mut fairings: Vec<_> = rocket.fairings_mut::<MyFairing>().collect();
    /// assert_eq!(fairings.len(), 3);
    /// assert_eq!(fairings[0].0, "some state");
    /// assert_eq!(fairings[1].0, "other state");
    /// assert_eq!(fairings[2].0, "yet more state");
    ///
    /// // we can modify the fairings
    /// fairings[1].0 = "modified state";
    ///
    /// let fairings: Vec<_> = rocket.fairings::<MyFairing>().collect();
    /// assert_eq!(fairings.len(), 3);
    /// assert_eq!(fairings[0].0, "some state");
    /// assert_eq!(fairings[1].0, "modified state");
    /// assert_eq!(fairings[2].0, "yet more state");
    /// ```
    pub fn fairings_mut<F: Fairing>(&mut self) -> impl Iterator<Item = &mut F> {
        match self.0.as_mut() {
            StateRefMut::Build(p) => Either::Left(p.fairings.filter_mut::<F>()),
            StateRefMut::Ignite(p) => Either::Right(p.fairings.filter_mut::<F>()),
            StateRefMut::Orbit(p) => Either::Right(p.fairings.filter_mut::<F>()),
        }
    }

    /// Returns the figment derived from the configuration provider set for
    /// `self`. To extract a typed config, prefer to use
    /// [`AdHoc::config()`](crate::fairing::AdHoc::config()).
    ///
    /// Note; A [`Figment`] generated from the current `provider` can _always_
    /// be retrieved via this method. However, because the provider can be
    /// changed at any point prior to ignition, a [`Config`] can only be
    /// retrieved in the ignite or orbit phases, or by manually extracting one
    /// from a particular figment.
    ///
    /// # Example
    ///
    /// ```rust
    /// let rocket = rocket::build();
    /// let figment = rocket.figment();
    /// ```
    pub fn figment(&self) -> &Figment {
        match self.0.as_ref() {
            StateRef::Build(p) => &p.figment,
            StateRef::Ignite(p) => &p.figment,
            StateRef::Orbit(p) => &p.figment,
        }
    }

    async fn into_ignite(self) -> Result<Rocket<Ignite>, Error> {
        match self.0.into_state() {
            State::Build(s) => Rocket::from(s).ignite().await,
            State::Ignite(s) => Ok(Rocket::from(s)),
            State::Orbit(s) => Ok(Rocket::from(s).deorbit()),
        }
    }

    pub(crate) async fn local_launch(self, e: Endpoint) -> Result<Rocket<Orbit>, Error> {
        Ok(self.into_ignite().await?._local_launch(e).await)
    }

    /// Returns a `Future` that transitions this instance of `Rocket` from any
    /// phase into the _orbit_ phase. When `await`ed, the future drives the
    /// server forward, listening for and dispatching requests to mounted routes
    /// and catchers.
    ///
    /// In addition to all of the processes that occur during
    /// [ignition](Rocket::ignite()), a successful launch results in _liftoff_
    /// fairings being executed _after_ binding to any respective network
    /// interfaces but before serving the first request. Liftoff fairings are
    /// run concurrently; resolution of all fairings is `await`ed before
    /// resuming request serving.
    ///
    /// The `Future` resolves as an `Err` if any of the following occur:
    ///
    ///   * there is an error igniting; see [`Rocket::ignite()`].
    ///   * there is an I/O error starting the server.
    ///   * an unrecoverable, system-level error occurs while running.
    ///
    /// The `Future` resolves as an `Ok` if any of the following occur:
    ///
    ///   * graceful shutdown via [`Shutdown::notify()`] completes.
    ///
    /// The returned value on `Ok(())` is previously running instance.
    ///
    /// The `Future` does not resolve otherwise.
    ///
    /// # Error
    ///
    /// If there is a problem starting the application or the application fails
    /// unexpectedly while running, an [`Error`] is returned. Note that a value
    /// of type `Error` panics if dropped without first being inspected. See the
    /// [`Error`] documentation for more information.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// #[rocket::main]
    /// async fn main() {
    ///     let result = rocket::build().launch().await;
    ///
    ///     // this is reachable only after `Shutdown::notify()` or `Ctrl+C`.
    ///     println!("Rocket: deorbit.");
    /// }
    /// ```
    pub async fn launch(self) -> Result<Rocket<Ignite>, Error> {
        self.launch_with::<DefaultListener>().await
    }

    pub async fn launch_with<B: Bind>(self) -> Result<Rocket<Ignite>, Error> {
        let rocket = self.into_ignite().await?;
        let bind_endpoint = B::bind_endpoint(&rocket).ok();
        let listener: B = B::bind(&rocket).await
            .map_err(|e| ErrorKind::Bind(bind_endpoint, Box::new(e)))?;

        let any: Box<dyn Any + Send + Sync> = Box::new(listener);
        match any.downcast::<DefaultListener>() {
            Ok(listener) => {
                let listener = *listener;
                crate::util::for_both!(listener, listener => {
                    crate::util::for_both!(listener, listener => {
                        rocket._launch(listener).await
                    })
                })
            }
            Err(any) => {
                let listener = *any.downcast::<B>().unwrap();
                rocket._launch(listener).await
            }
        }
    }

    pub async fn try_launch_on<L, F, E>(self, listener: F) -> Result<Rocket<Ignite>, Error>
        where L: Listener + 'static,
              F: Future<Output = Result<L, E>>,
              E: std::error::Error + Send + 'static
    {
        let listener = listener.map_err(|e| ErrorKind::Bind(None, Box::new(e))).await?;
        self.into_ignite().await?._launch(listener).await
    }

    pub async fn launch_on<L>(self, listener: L) -> Result<Rocket<Ignite>, Error>
        where L: Listener + 'static,
    {
        self.into_ignite().await?._launch(listener).await
    }
}

#[doc(hidden)]
impl<P: Phase> Deref for Rocket<P> {
    type Target = P::State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[doc(hidden)]
impl<P: Phase> DerefMut for Rocket<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: Phase> fmt::Debug for Rocket<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
