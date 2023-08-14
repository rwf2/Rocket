use futures::future::{Future, BoxFuture, FutureExt};
use parking_lot::Mutex;

use crate::route::RouteUri;
use crate::{Rocket, Request, Response, Data, Build, Orbit};
use crate::fairing::{Fairing, Kind, Info, Result};

/// A ad-hoc fairing that can be created from a function or closure.
///
/// This enum can be used to create a fairing from a simple function or closure
/// without creating a new structure or implementing `Fairing` directly.
///
/// # Usage
///
/// Use [`AdHoc::on_ignite`], [`AdHoc::on_liftoff`], [`AdHoc::on_request()`], or
/// [`AdHoc::on_response()`] to create an `AdHoc` structure from a function or
/// closure. Then, simply attach the structure to the `Rocket` instance.
///
/// # Example
///
/// The following snippet creates a `Rocket` instance with two ad-hoc fairings.
/// The first, a liftoff fairing named "Liftoff Printer", simply prints a message
/// indicating that Rocket has launched. The second named "Put Rewriter", a
/// request fairing, rewrites the method of all requests to be `PUT`.
///
/// ```rust
/// use rocket::fairing::AdHoc;
/// use rocket::http::Method;
///
/// rocket::build()
///     .attach(AdHoc::on_liftoff("Liftoff Printer", |_| Box::pin(async move {
///         println!("...annnddd we have liftoff!");
///     })))
///     .attach(AdHoc::on_request("Put Rewriter", |req, _| Box::pin(async move {
///         req.set_method(Method::Put);
///     })));
/// ```
pub struct AdHoc {
    name: &'static str,
    kind: AdHocKind,
}

struct Once<F: ?Sized>(Mutex<Option<Box<F>>>);

impl<F: ?Sized> Once<F> {
    fn new(f: Box<F>) -> Self { Once(Mutex::new(Some(f))) }

    #[track_caller]
    fn take(&self) -> Box<F> {
        self.0.lock().take().expect("Once::take() called once")
    }
}

enum AdHocKind {
    /// An ad-hoc **ignite** fairing. Called during ignition.
    Ignite(Once<dyn FnOnce(Rocket<Build>) -> BoxFuture<'static, Result> + Send + 'static>),

    /// An ad-hoc **liftoff** fairing. Called just after Rocket launches.
    Liftoff(Once<dyn for<'a> FnOnce(&'a Rocket<Orbit>) -> BoxFuture<'a, ()> + Send + 'static>),

    /// An ad-hoc **request** fairing. Called when a request is received.
    Request(Box<dyn for<'a> Fn(&'a mut Request<'_>, &'a Data<'_>)
        -> BoxFuture<'a, ()> + Send + Sync + 'static>),

    /// An ad-hoc **response** fairing. Called when a response is ready to be
    /// sent to a client.
    Response(Box<dyn for<'r, 'b> Fn(&'r Request<'_>, &'b mut Response<'r>)
        -> BoxFuture<'b, ()> + Send + Sync + 'static>),

    /// An ad-hoc **shutdown** fairing. Called on shutdown.
    Shutdown(Once<dyn for<'a> FnOnce(&'a Rocket<Orbit>) -> BoxFuture<'a, ()> + Send + 'static>),
}

impl AdHoc {
    /// Constructs an `AdHoc` ignite fairing named `name`. The function `f` will
    /// be called by Rocket during the [`Rocket::ignite()`] phase.
    ///
    /// This version of an `AdHoc` ignite fairing cannot abort ignite. For a
    /// fallible version that can, see [`AdHoc::try_on_ignite()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op ignite fairing.
    /// let fairing = AdHoc::on_ignite("Boom!", |rocket| async move {
    ///     rocket
    /// });
    /// ```
    pub fn on_ignite<F, Fut>(name: &'static str, f: F) -> AdHoc
        where F: FnOnce(Rocket<Build>) -> Fut + Send + 'static,
              Fut: Future<Output = Rocket<Build>> + Send + 'static,
    {
        AdHoc::try_on_ignite(name, |rocket| f(rocket).map(Ok))
    }

    /// Constructs an `AdHoc` ignite fairing named `name`. The function `f` will
    /// be called by Rocket during the [`Rocket::ignite()`] phase. Returning an
    /// `Err` aborts ignition and thus launch.
    ///
    /// For an infallible version, see [`AdHoc::on_ignite()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op try ignite fairing.
    /// let fairing = AdHoc::try_on_ignite("No-Op", |rocket| async { Ok(rocket) });
    /// ```
    pub fn try_on_ignite<F, Fut>(name: &'static str, f: F) -> AdHoc
        where F: FnOnce(Rocket<Build>) -> Fut + Send + 'static,
              Fut: Future<Output = Result> + Send + 'static,
    {
        AdHoc { name, kind: AdHocKind::Ignite(Once::new(Box::new(|r| f(r).boxed()))) }
    }

    /// Constructs an `AdHoc` liftoff fairing named `name`. The function `f`
    /// will be called by Rocket just after [`Rocket::launch()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // A fairing that prints a message just before launching.
    /// let fairing = AdHoc::on_liftoff("Boom!", |_| Box::pin(async move {
    ///     println!("Rocket has lifted off!");
    /// }));
    /// ```
    pub fn on_liftoff<F: Send + Sync + 'static>(name: &'static str, f: F) -> AdHoc
        where F: for<'a> FnOnce(&'a Rocket<Orbit>) -> BoxFuture<'a, ()>
    {
        AdHoc { name, kind: AdHocKind::Liftoff(Once::new(Box::new(f))) }
    }

    /// Constructs an `AdHoc` request fairing named `name`. The function `f`
    /// will be called and the returned `Future` will be `await`ed by Rocket
    /// when a new request is received.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op request fairing.
    /// let fairing = AdHoc::on_request("Dummy", |req, data| {
    ///     Box::pin(async move {
    ///         // do something with the request and data...
    /// #       let (_, _) = (req, data);
    ///     })
    /// });
    /// ```
    pub fn on_request<F: Send + Sync + 'static>(name: &'static str, f: F) -> AdHoc
        where F: for<'a> Fn(&'a mut Request<'_>, &'a Data<'_>) -> BoxFuture<'a, ()>
    {
        AdHoc { name, kind: AdHocKind::Request(Box::new(f)) }
    }

    // FIXME(rustc): We'd like to allow passing `async fn` to these methods...
    // https://github.com/rust-lang/rust/issues/64552#issuecomment-666084589

    /// Constructs an `AdHoc` response fairing named `name`. The function `f`
    /// will be called and the returned `Future` will be `await`ed by Rocket
    /// when a response is ready to be sent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op response fairing.
    /// let fairing = AdHoc::on_response("Dummy", |req, resp| {
    ///     Box::pin(async move {
    ///         // do something with the request and pending response...
    /// #       let (_, _) = (req, resp);
    ///     })
    /// });
    /// ```
    pub fn on_response<F: Send + Sync + 'static>(name: &'static str, f: F) -> AdHoc
        where F: for<'b, 'r> Fn(&'r Request<'_>, &'b mut Response<'r>) -> BoxFuture<'b, ()>
    {
        AdHoc { name, kind: AdHocKind::Response(Box::new(f)) }
    }

    /// Constructs an `AdHoc` shutdown fairing named `name`. The function `f`
    /// will be called by Rocket when [shutdown is triggered].
    ///
    /// [shutdown is triggered]: crate::config::Shutdown#triggers
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // A fairing that prints a message just before launching.
    /// let fairing = AdHoc::on_shutdown("Bye!", |_| Box::pin(async move {
    ///     println!("Rocket is on its way back!");
    /// }));
    /// ```
    pub fn on_shutdown<F: Send + Sync + 'static>(name: &'static str, f: F) -> AdHoc
        where F: for<'a> FnOnce(&'a Rocket<Orbit>) -> BoxFuture<'a, ()>
    {
        AdHoc { name, kind: AdHocKind::Shutdown(Once::new(Box::new(f))) }
    }

    /// Constructs an `AdHoc` launch fairing that extracts a configuration of
    /// type `T` from the configured provider and stores it in managed state. If
    /// extractions fails, pretty-prints the error message and aborts launch.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::launch;
    /// use serde::Deserialize;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[derive(Deserialize)]
    /// struct Config {
    ///     field: String,
    ///     other: usize,
    ///     /* and so on.. */
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().attach(AdHoc::config::<Config>())
    /// }
    /// ```
    pub fn config<'de, T>() -> AdHoc
        where T: serde::Deserialize<'de> + Send + Sync + 'static
    {
        AdHoc::try_on_ignite(std::any::type_name::<T>(), |rocket| async {
            let app_config = match rocket.figment().extract::<T>() {
                Ok(config) => config,
                Err(e) => {
                    crate::config::pretty_print_error(e);
                    return Err(rocket);
                }
            };

            Ok(rocket.manage(app_config))
        })
    }

    /// Constructs an `AdHoc` request fairing that strips trailing slashes from
    /// all URIs in all incoming requests.
    ///
    /// The fairing returned by this method is intended largely for applications
    /// that migrated from Rocket v0.4 to Rocket v0.5. In Rocket v0.4, requests
    /// with a trailing slash in the URI were treated as if the trailing slash
    /// were not present. For example, the request URI `/foo/` would match the
    /// route `/<a>` with `a = foo`. If the application depended on this
    /// behavior, say by using URIs with previously innocuous trailing slashes
    /// in an external application, requests will not be routed as expected.
    ///
    /// This fairing resolves this issue by stripping a trailing slash, if any,
    /// in all incoming URIs. When it does so, it logs a warning. It is
    /// recommended to use this fairing as a stop-gap measure instead of a
    /// permanent resolution, if possible.
    //
    /// # Example
    ///
    /// With the fairing attached, request URIs have a trailing slash stripped:
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::local::blocking::Client;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[get("/<param>")]
    /// fn foo(param: &str) -> &str {
    ///     param
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/", routes![foo])
    ///         .attach(AdHoc::uri_normalizer())
    /// }
    ///
    /// # let client = Client::debug(rocket()).unwrap();
    /// let response = client.get("/bar/").dispatch();
    /// assert_eq!(response.into_string().unwrap(), "bar");
    /// ```
    ///
    /// Without it, request URIs are unchanged and routed normally:
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::local::blocking::Client;
    /// use rocket::fairing::AdHoc;
    ///
    /// #[get("/<param>")]
    /// fn foo(param: &str) -> &str {
    ///     param
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().mount("/", routes![foo])
    /// }
    ///
    /// # let client = Client::debug(rocket()).unwrap();
    /// let response = client.get("/bar/").dispatch();
    /// assert!(response.status().class().is_client_error());
    ///
    /// let response = client.get("/bar").dispatch();
    /// assert_eq!(response.into_string().unwrap(), "bar");
    /// ```
    // #[deprecated(since = "0.6", note = "routing from Rocket v0.5 is now standard")]
    pub fn uri_normalizer() -> impl Fairing {
        #[derive(Default)]
        struct Normalizer {
            routes: state::InitCell<Vec<crate::Route>>,
        }

        impl Normalizer {
            fn routes(&self, rocket: &Rocket<Orbit>) -> &[crate::Route] {
                self.routes.get_or_init(|| {
                    rocket.routes()
                        .filter(|r| r.uri.has_trailing_slash())
                        .cloned()
                        .collect()
                })
            }
        }

        #[crate::async_trait]
        impl Fairing for Normalizer {
            fn info(&self) -> Info {
                Info { name: "URI Normalizer", kind: Kind::Ignite | Kind::Liftoff | Kind::Request }
            }

            async fn on_ignite(&self, rocket: Rocket<Build>) -> Result {
                // We want a route like `/foo/<bar..>` to match a request for
                // `/foo` as it would have before. While we could check if a
                // route is mounted that would cause this match and then rewrite
                // the request URI as `/foo/`, doing so is expensive and
                // potentially incorrect due to request guards and ranking.
                //
                // Instead, we generate a new route with URI `/foo` with the
                // same rank and handler as the `/foo/<bar..>` route and mount
                // it to this instance of `rocket`. This preserves the previous
                // matching while still checking request guards.
                let normalized_trailing = rocket.routes()
                    .filter(|r| r.uri.metadata.dynamic_trail)
                    .filter(|r| r.uri.path().segments().num() > 1)
                    .filter_map(|route| {
                        let path = route.uri.unmounted().path();
                        let new_path = path.as_str()
                            .rsplit_once('/')
                            .map(|(prefix, _)| prefix)
                            .filter(|path| !path.is_empty())
                            .unwrap_or("/");

                        let base = route.uri.base().as_str();
                        let uri = match route.uri.unmounted().query() {
                            Some(q) => format!("{}?{}", new_path, q),
                            None => new_path.to_string()
                        };

                        let mut route = route.clone();
                        route.uri = RouteUri::try_new(base, &uri).ok()?;
                        route.name = route.name.map(|r| format!("{} [normalized]", r).into());
                        Some(route)
                    })
                    .collect::<Vec<_>>();

                Ok(rocket.mount("/", normalized_trailing))
            }

            async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
                let _ = self.routes(rocket);
            }

            async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
                // If the URI has no trailing slash, it routes as before.
                if req.uri().is_normalized_nontrailing() {
                    return
                }

                // Otherwise, check if there's a route that matches the request
                // with a trailing slash. If there is, leave the request alone.
                // This allows incremental compatibility updates. Otherwise,
                // rewrite the request URI to remove the `/`.
                if !self.routes(req.rocket()).iter().any(|r| r.matches(req)) {
                    let normal = req.uri().clone().into_normalized_nontrailing();
                    warn!("Incoming request URI was normalized for compatibility.");
                    info_!("{} -> {}", req.uri(), normal);
                    req.set_uri(normal);
                }
            }
        }

        Normalizer::default()
    }
}

#[crate::async_trait]
impl Fairing for AdHoc {
    fn info(&self) -> Info {
        let kind = match self.kind {
            AdHocKind::Ignite(_) => Kind::Ignite,
            AdHocKind::Liftoff(_) => Kind::Liftoff,
            AdHocKind::Request(_) => Kind::Request,
            AdHocKind::Response(_) => Kind::Response,
            AdHocKind::Shutdown(_) => Kind::Shutdown,
        };

        Info { name: self.name, kind }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result {
        match self.kind {
            AdHocKind::Ignite(ref f) => (f.take())(rocket).await,
            _ => Ok(rocket)
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        if let AdHocKind::Liftoff(ref f) = self.kind {
            (f.take())(rocket).await
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, data: &mut Data<'_>) {
        if let AdHocKind::Request(ref f) = self.kind {
            f(req, data).await
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        if let AdHocKind::Response(ref f) = self.kind {
            f(req, res).await
        }
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        if let AdHocKind::Shutdown(ref f) = self.kind {
            (f.take())(rocket).await
        }
    }
}
