use std::sync::Mutex;

use futures::future::{Future, BoxFuture, FutureExt};

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
        self.0.lock().expect("Once::lock()").take().expect("Once::take() called once")
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
