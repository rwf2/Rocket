use std::fmt::Display;
use std::convert::TryInto;

use yansi::Paint;
use state::Container;
use figment::Figment;
use tokio::sync::mpsc;
use futures::future::FutureExt;

use crate::logger;
use crate::config::Config;
use crate::catcher::Catcher;
use crate::router::{Router, Route};
use crate::fairing::{Fairing, Fairings};
use crate::logger::PaintExt;
use crate::shutdown::Shutdown;
use crate::http::{uri::Origin, ext::IntoOwned};
use crate::error::{Error, ErrorKind};

/// The main `Rocket` type: used to mount routes and catchers and launch the
/// application.
#[derive(Debug)]
pub struct Rocket {
    pub(crate) config: Config,
    pub(crate) figment: Figment,
    pub(crate) managed_state: Container![Send + Sync],
    pub(crate) router: Router,
    pub(crate) fairings: Fairings,
    pub(crate) shutdown_receiver: Option<mpsc::Receiver<()>>,
    pub(crate) shutdown_handle: Shutdown,
}

impl Rocket {
    /// Create a new `Rocket` application using the configuration information in
    /// `Rocket.toml`. If the file does not exist or if there is an I/O error
    /// reading the file, the defaults, overridden by any environment-based
    /// parameters, are used. See the [`config`](crate::config) documentation
    /// for more information on defaults.
    ///
    /// This method is typically called through the
    /// [`rocket::ignite()`](crate::ignite) alias.
    ///
    /// # Panics
    ///
    /// If there is an error reading configuration sources, this function prints
    /// a nice error message and then exits the process.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # {
    /// rocket::ignite()
    /// # };
    /// ```
    #[track_caller]
    #[inline(always)]
    pub fn ignite() -> Rocket {
        Rocket::custom(Config::figment())
    }

    /// Creates a new `Rocket` application using the supplied configuration
    /// provider. This method is typically called through the
    /// [`rocket::custom()`](crate::custom()) alias.
    ///
    /// # Panics
    ///
    /// If there is an error reading a [`Config`] from `provider`, function
    /// prints a nice error message and then exits the process.
    ///
    /// # Example
    ///
    /// ```rust
    /// use figment::{Figment, providers::{Toml, Env, Format}};
    ///
    /// #[rocket::launch]
    /// fn rocket() -> _ {
    ///     let figment = Figment::from(rocket::Config::default())
    ///         .merge(Toml::file("MyApp.toml").nested())
    ///         .merge(Env::prefixed("MY_APP_").global());
    ///
    ///     rocket::custom(figment)
    /// }
    /// ```
    #[track_caller]
    pub fn custom<T: figment::Provider>(provider: T) -> Rocket {
        let config = Config::from(&provider);
        let figment = Figment::from(provider);
        logger::init(&config);
        config.pretty_print(&figment);

        let managed_state = <Container![Send + Sync]>::new();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);
        Rocket {
            config, figment, managed_state,
            shutdown_handle: Shutdown(shutdown_sender),
            router: Router::new(),
            fairings: Fairings::new(),
            shutdown_receiver: Some(shutdown_receiver),
        }
    }

    /// Resets the configuration in `self` to that provided by `provider`.
    ///
    /// # Panics
    ///
    /// Like [`Rocket::custom()`], panics if `provider` does not provide a valid
    /// [`Config`]. The error message is printed.
    ///
    /// # Examples
    ///
    /// To modify only some values, use the existing `config`:
    ///
    /// ```rust
    /// use std::net::Ipv4Addr;
    ///
    /// let config = rocket::Config {
    ///     port: 7777,
    ///     address: Ipv4Addr::new(18, 127, 0, 1).into(),
    ///     ..rocket::Config::default()
    /// };
    ///
    /// let rocket = rocket::custom(&config);
    /// assert_eq!(rocket.config().port, 7777);
    /// assert_eq!(rocket.config().address, Ipv4Addr::new(18, 127, 0, 1));
    ///
    /// // Modifying the existing config:
    /// let mut new_config = rocket.config().clone();
    /// new_config.port = 8888;
    ///
    /// let rocket = rocket.reconfigure(new_config);
    /// assert_eq!(rocket.config().port, 8888);
    /// assert_eq!(rocket.config().address, Ipv4Addr::new(18, 127, 0, 1));
    ///
    /// // Modifying the existing figment:
    /// let mut new_figment = rocket.figment().clone()
    ///     .merge(("address", "171.64.200.10"));
    ///
    /// let rocket = rocket.reconfigure(new_figment);
    /// assert_eq!(rocket.config().port, 8888);
    /// assert_eq!(rocket.config().address, Ipv4Addr::new(171, 64, 200, 10));
    /// ```
    #[inline]
    #[track_caller]
    pub fn reconfigure<T: figment::Provider>(mut self, provider: T) -> Rocket {
        self.config = Config::from(&provider);
        self.figment = Figment::from(provider);
        logger::init(&self.config);
        self.config.pretty_print(&self.figment);
        self
    }

    /// Mounts all of the routes in the supplied vector at the given `base`
    /// path. Mounting a route with path `path` at path `base` makes the route
    /// available at `base/path`.
    ///
    /// # Panics
    ///
    /// Panics if the `base` mount point is not a valid static path: a valid
    /// origin URI without dynamic parameters.
    ///
    /// Panics if any route's URI is not a valid origin URI. This kind of panic
    /// is guaranteed not to occur if the routes were generated using Rocket's
    /// code generation.
    ///
    /// # Examples
    ///
    /// Use the `routes!` macro to mount routes created using the code
    /// generation facilities. Requests to the `/hello/world` URI will be
    /// dispatched to the `hi` route.
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
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite().mount("/hello", routes![hi])
    /// }
    /// ```
    ///
    /// Manually create a route named `hi` at path `"/world"` mounted at base
    /// `"/hello"`. Requests to the `/hello/world` URI will be dispatched to the
    /// `hi` route.
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::{HandlerFuture, Outcome};
    /// use rocket::http::Method::*;
    ///
    /// fn hi<'r>(req: &'r Request, _: Data) -> HandlerFuture<'r> {
    ///     Outcome::from(req, "Hello!").pin()
    /// }
    ///
    /// # let _ = async { // We don't actually want to launch the server in an example.
    /// rocket::ignite().mount("/hello", vec![Route::new(Get, "/world", hi)])
    /// #     .launch().await;
    /// # };
    /// ```
    pub fn mount<'a, B, R>(mut self, base: B, routes: R) -> Self
        where B: TryInto<Origin<'a>> + Clone + Display,
              B::Error: Display,
              R: Into<Vec<Route>>
    {
        let base_uri = base.clone().try_into()
            .map(|origin| origin.into_owned())
            .unwrap_or_else(|e| {
                error!("Invalid route base: {}.", Paint::white(&base));
                panic!("Error: {}", e);
            });

        if base_uri.query().is_some() {
            error!("Mount point '{}' contains query string.", base);
            panic!("Invalid mount point.");
        }

        info!("{}{} {} {}",
              Paint::emoji("🛰  "),
              Paint::magenta("Mounting"),
              Paint::blue(&base_uri),
              Paint::magenta("routes:"));

        for route in routes.into() {
            let mounted_route = route.clone()
                .map_base(|old| format!("{}{}", base, old))
                .unwrap_or_else(|e| {
                    error_!("Route `{}` has a malformed URI.", route);
                    error_!("{}", e);
                    panic!("Invalid route URI.");
                });

            info_!("{}", mounted_route);
            self.router.add_route(mounted_route);
        }

        self
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
    /// #[catch(400)]
    /// fn not_found(req: &Request) -> String {
    ///     format!("I couldn't find '{}'. Try something else?", req.uri())
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite().register("/", catchers![internal_error, not_found])
    /// }
    /// ```
    pub fn register<'a, B, C>(mut self, base: B, catchers: C) -> Self
        where B: TryInto<Origin<'a>> + Clone + Display,
              B::Error: Display,
              C: Into<Vec<Catcher>>
    {
        info!("{}{} {} {}",
              Paint::emoji("👾 "),
              Paint::magenta("Registering"),
              Paint::blue(&base),
              Paint::magenta("catchers:"));

        for catcher in catchers.into() {
            let mounted_catcher = catcher.clone()
                .map_base(|old| format!("{}{}", base, old))
                .unwrap_or_else(|e| {
                    error_!("Catcher `{}` has a malformed URI.", catcher);
                    error_!("{}", e);
                    panic!("Invalid catcher URI.");
                });

            info_!("{}", mounted_catcher);
            self.router.add_catcher(mounted_catcher);
        }

        self
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
    /// struct MyValue(usize);
    ///
    /// #[get("/")]
    /// fn index(state: State<MyValue>) -> String {
    ///     format!("The stateful value is: {}", state.0)
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .mount("/", routes![index])
    ///         .manage(MyValue(10))
    /// }
    /// ```
    #[inline]
    pub fn manage<T: Send + Sync + 'static>(self, state: T) -> Self {
        let type_name = std::any::type_name::<T>();
        if !self.managed_state.set(state) {
            error!("State for type '{}' is already being managed!", type_name);
            panic!("Aborting due to duplicately managed state.");
        }

        self
    }

    /// Attaches a fairing to this instance of Rocket. If the fairing is an
    /// _attach_ fairing, it is run immediately. All other kinds of fairings
    /// will be executed at their appropriate time.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Launch Message", |_| {
    ///             println!("Rocket is launching!");
    ///         }))
    /// }
    /// ```
    pub fn attach<F: Fairing>(mut self, fairing: F) -> Self {
        let future = async move {
            let fairing = Box::new(fairing);
            let mut fairings = std::mem::replace(&mut self.fairings, Fairings::new());
            let rocket = fairings.attach(fairing, self).await;
            (rocket, fairings)
        };

        // TODO: Reuse a single thread to run all attach fairings.
        let (rocket, mut fairings) = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                std::thread::spawn(move || {
                    let _e = handle.enter();
                    futures::executor::block_on(future)
                }).join().unwrap()
            }
            Err(_) => {
                std::thread::spawn(|| {
                    futures::executor::block_on(future)
                }).join().unwrap()
            }
        };

        self = rocket;

        // Note that `self.fairings` may now be non-empty! Move them to the end.
        fairings.append(self.fairings);
        self.fairings = fairings;
        self
    }

    /// Returns the active configuration.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Config Printer", |rocket| {
    ///             println!("Rocket launch config: {:?}", rocket.config());
    ///         }))
    /// }
    /// ```
    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns the figment for configured provider.
    ///
    /// # Example
    ///
    /// ```rust
    /// let rocket = rocket::ignite();
    /// let figment = rocket.figment();
    ///
    /// let port: u16 = figment.extract_inner("port").unwrap();
    /// assert_eq!(port, rocket.config().port);
    /// ```
    #[inline(always)]
    pub fn figment(&self) -> &Figment {
        &self.figment
    }

    /// Returns an iterator over all of the routes mounted on this instance of
    /// Rocket. The order is unspecified.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[get("/hello")]
    /// fn hello() -> &'static str {
    ///     "Hello, world!"
    /// }
    ///
    /// fn main() {
    ///     let mut rocket = rocket::ignite()
    ///         .mount("/", routes![hello])
    ///         .mount("/hi", routes![hello]);
    ///
    ///     for route in rocket.routes() {
    ///         match route.uri.base() {
    ///             "/" => assert_eq!(route.uri.path(), "/hello"),
    ///             "/hi" => assert_eq!(route.uri.path(), "/hi/hello"),
    ///             _ => unreachable!("only /hello, /hi/hello are expected")
    ///         }
    ///     }
    ///
    ///     assert_eq!(rocket.routes().count(), 2);
    /// }
    /// ```
    #[inline(always)]
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.router.routes()
    }

    /// Returns an iterator over all of the catchers registered on this instance
    /// of Rocket. The order is unspecified.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[catch(404)] fn not_found() -> &'static str { "Nothing here, sorry!" }
    /// #[catch(500)] fn just_500() -> &'static str { "Whoops!?" }
    /// #[catch(default)] fn some_default() -> &'static str { "Everything else." }
    ///
    /// fn main() {
    ///     let mut rocket = rocket::ignite()
    ///         .register("/", catchers![not_found, just_500, some_default]);
    ///
    ///     let mut codes: Vec<_> = rocket.catchers().map(|c| c.code).collect();
    ///     codes.sort();
    ///
    ///     assert_eq!(codes, vec![None, Some(404), Some(500)]);
    /// }
    /// ```
    #[inline(always)]
    pub fn catchers(&self) -> impl Iterator<Item = &Catcher> {
        self.router.catchers()
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
    /// let rocket = rocket::ignite().manage(MyState("hello!"));
    /// assert_eq!(rocket.state::<MyState>(), Some(&MyState("hello!")));
    /// ```
    #[inline(always)]
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.managed_state.try_get()
    }

    /// Returns a handle which can be used to gracefully terminate this instance
    /// of Rocket. In routes, use the [`Shutdown`] request guard.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use std::{thread, time::Duration};
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite();
    /// let handle = rocket.shutdown();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(10));
    ///     handle.shutdown();
    /// });
    ///
    /// // Shuts down after 10 seconds
    /// let shutdown_result = rocket.launch().await;
    /// assert!(shutdown_result.is_ok());
    /// # });
    /// ```
    #[inline(always)]
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown_handle.clone()
    }

    /// Perform "pre-launch" checks: verify:
    ///     * there are no routing colisionns
    ///     * there were no fairing failures
    ///     * a secret key, if needed, is securely configured
    pub(crate) async fn prelaunch_check(&mut self) -> Result<(), Error> {
        if let Err(collisions) = self.router.finalize() {
            return Err(Error::new(ErrorKind::Collisions(collisions)));
        }

        if let Some(failures) = self.fairings.failures() {
            return Err(Error::new(ErrorKind::FailedFairings(failures.to_vec())))
        }

        #[cfg(feature = "secrets")]
        if !self.config.secret_key.is_provided() {
            let profile = self.figment.profile();
            if profile != Config::DEBUG_PROFILE {
                return Err(Error::new(ErrorKind::InsecureSecretKey(profile.clone())));
            } else if self.config.secret_key.is_zero() {
                self.config.secret_key = crate::config::SecretKey::generate()
                    .unwrap_or(crate::config::SecretKey::zero());

                warn!("secrets enabled without a stable `secret_key`");
                info_!("disable `secrets` feature or configure a `secret_key`");
                info_!("this becomes an {} in non-debug profiles", Paint::red("error"));

                if !self.config.secret_key.is_zero() {
                    warn_!("a random key has been generated for this launch");
                }
            }
        };

        Ok(())
    }

    /// Returns a `Future` that drives the server, listening for and dispatching
    /// requests to mounted routes and catchers. The `Future` completes when the
    /// server is shut down via [`Shutdown`], encounters a fatal error, or if
    /// the the `ctrlc` configuration option is set, when `Ctrl+C` is pressed.
    ///
    /// # Error
    ///
    /// If there is a problem starting the application, an [`Error`] is
    /// returned. Note that a value of type `Error` panics if dropped without
    /// first being inspected. See the [`Error`] documentation for more
    /// information.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[rocket::main]
    /// async fn main() {
    /// # if false {
    ///     let result = rocket::ignite().launch().await;
    ///     assert!(result.is_ok());
    /// # }
    /// }
    /// ```
    pub async fn launch(mut self) -> Result<(), Error> {
        use std::net::ToSocketAddrs;
        use futures::future::Either;
        use crate::http::private::bind_tcp;

        self.prelaunch_check().await?;

        let full_addr = format!("{}:{}", self.config.address, self.config.port);
        let addr = full_addr.to_socket_addrs()
            .map(|mut addrs| addrs.next().expect(">= 1 socket addr"))
            .map_err(|e| Error::new(ErrorKind::Io(e)))?;

        // If `ctrl-c` shutdown is enabled, we `select` on `the ctrl-c` signal
        // and server. Otherwise, we only wait on the `server`, hence `pending`.
        let shutdown_handle = self.shutdown_handle.clone();
        let shutdown_signal = match self.config.ctrlc {
            true => tokio::signal::ctrl_c().boxed(),
            false => futures::future::pending().boxed(),
        };

        #[cfg(feature = "tls")]
        let server = {
            use crate::http::private::tls::{bind_tls, ProtocolVersion, ciphersuite};

            if let Some(tls_config) = &self.config.tls {
                let (certs, key) = tls_config.to_readers().map_err(ErrorKind::Io)?;

                let ciphersuites: Vec<_> = tls_config.v13_ciphers.iter().map(|c| {
                    use crate::config::V13Ciphers::*;

                    match c {
                        Chacha20Poly1305Sha256 => &ciphersuite::TLS13_CHACHA20_POLY1305_SHA256,
                        Aes256GcmSha384 => &ciphersuite::TLS13_AES_256_GCM_SHA384,
                        Aes128GcmSha256 => &ciphersuite::TLS13_AES_128_GCM_SHA256,
                    }
                }).chain(tls_config.v12_ciphers.iter().map(|c| {
                    use crate::config::V12Ciphers::*;

                    match c {
                        EcdheEcdsaWithChacha20Poly1305Sha256 => &ciphersuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                        EcdheRsaWithChacha20Poly1305Sha256 => &ciphersuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                        EcdheEcdsaWithAes256GcmSha384 => &ciphersuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                        EcdheEcdsaWithAes128GcmSha256 => &ciphersuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                        EcdheRsaWithAes256GcmSha384 => &ciphersuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                        EcdheRsaWithAes128GcmSha256 => &ciphersuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
                    }
                })).collect();

                if ciphersuites.is_empty() {
                    unreachable!("Ensuring that ciphersuites is non-empty happens at configure time");
                }

                let mut versions = vec![];

                if !tls_config.v12_ciphers.is_empty() {
                    versions.push(ProtocolVersion::TLSv1_2);
                }

                if !tls_config.v13_ciphers.is_empty() {
                    versions.push(ProtocolVersion::TLSv1_3);
                }

                let l = bind_tls(addr, certs, key, ciphersuites, versions, tls_config.prefer_server_ciphers_order).await.map_err(ErrorKind::Bind)?;
                self.listen_on(l).boxed()
            } else {
                let l = bind_tcp(addr).await.map_err(ErrorKind::Bind)?;
                self.listen_on(l).boxed()
            }
        };

        #[cfg(not(feature = "tls"))]
        let server = {
            let l = bind_tcp(addr).await.map_err(ErrorKind::Bind)?;
            self.listen_on(l).boxed()
        };

        match futures::future::select(shutdown_signal, server).await {
            Either::Left((Ok(()), server)) => {
                // Ctrl-was pressed. Signal shutdown, wait for the server.
                shutdown_handle.shutdown();
                server.await
            }
            Either::Left((Err(err), server)) => {
                // Error setting up ctrl-c signal. Let the user know.
                warn!("Failed to enable `ctrl-c` graceful signal shutdown.");
                info_!("Error: {}", err);
                server.await
            }
            // Server shut down before Ctrl-C; return the result.
            Either::Right((result, _)) => result,
        }
    }
}
