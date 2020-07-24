use std::sync::Mutex;

use futures::future::{Future, BoxFuture};

use crate::{Cargo, Rocket, Request, Response, Data};
use crate::fairing::{Fairing, Kind, Info};

/// A ad-hoc fairing that can be created from a function or closure.
///
/// This enum can be used to create a fairing from a simple function or closure
/// without creating a new structure or implementing `Fairing` directly.
///
/// # Usage
///
/// Use the [`on_attach`](#method.on_attach), [`on_launch`](#method.on_launch),
/// [`on_request`](#method.on_request), or [`on_response`](#method.on_response)
/// constructors to create an `AdHoc` structure from a function or closure.
/// Then, simply attach the structure to the `Rocket` instance.
///
/// # Example
///
/// The following snippet creates a `Rocket` instance with two ad-hoc fairings.
/// The first, a launch fairing named "Launch Printer", simply prints a message
/// indicating that the application is about to the launch. The second named
/// "Put Rewriter", a request fairing, rewrites the method of all requests to be
/// `PUT`.
///
/// ```rust
/// use rocket::fairing::AdHoc;
/// use rocket::http::Method;
///
/// rocket::ignite()
///     .attach(AdHoc::on_launch("Launch Printer", |_| {
///         println!("Rocket is about to launch! Exciting! Here we go...");
///     }))
///     .attach(AdHoc::on_request("Put Rewriter", |req, _| {
///         Box::pin(async move {
///             req.set_method(Method::Put);
///         })
///     }));
/// ```
pub struct AdHoc {
    name: &'static str,
    kind: AdHocKind,
}

// macro_rules! Async {
//     ($kind:ident <$l:lifetime> ($($param:ty),*) -> $r:ty) => (
//         dyn for<$l> $kind($($param),*) -> futures::future::BoxFuture<$l, $r>
//             + Send + 'static
//     );
//     ($kind:ident ($($param:ty),*) -> $r:ty) => (
//         dyn $kind($($param),*) -> futures::future::BoxFuture<'static, $r>
//             + Send + Sync + 'static
//     );
//     ($kind:ident <$l:lifetime> ($($param:ty),*)) => (
//         Async!($kind <$l> ($($param),*) -> ())
//     );
//     ($kind:ident ($($param:ty),*)) => (
//         Async!($kind ($($param),*) -> ())
//     );
// }

enum AdHocKind {
    /// An ad-hoc **attach** fairing. Called when the fairing is attached.
    Attach(Mutex<Option<Box<dyn FnOnce(Rocket)
        -> BoxFuture<'static, Result<Rocket, Rocket>> + Send + 'static>>>),

    /// An ad-hoc **launch** fairing. Called just before Rocket launches.
    Launch(Mutex<Option<Box<dyn FnOnce(&Cargo) + Send + 'static>>>),

    /// An ad-hoc **request** fairing. Called when a request is received.
    Request(Box<dyn for<'a> Fn(&'a mut Request<'_>, &'a Data)
        -> BoxFuture<'a, ()> + Send + Sync + 'static>),

    /// An ad-hoc **response** fairing. Called when a response is ready to be
    /// sent to a client.
    Response(Box<dyn for<'a> Fn(&'a Request<'_>, &'a mut Response<'_>)
        -> BoxFuture<'a, ()> + Send + Sync + 'static>),
}

impl AdHoc {
    /// Constructs an `AdHoc` attach fairing named `name`. The function `f` will
    /// be called by Rocket when this fairing is attached.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op attach fairing.
    /// let fairing = AdHoc::on_attach("No-Op", |rocket| async { Ok(rocket) });
    /// ```
    pub fn on_attach<F, Fut>(name: &'static str, f: F) -> AdHoc
    where
        F: FnOnce(Rocket) -> Fut + Send + 'static,
        Fut: Future<Output=Result<Rocket, Rocket>> + Send + 'static,
    {
        AdHoc {
            name,
            kind: AdHocKind::Attach(Mutex::new(Some(Box::new(|rocket| Box::pin(f(rocket))))))
        }
    }

    /// Constructs an `AdHoc` launch fairing named `name`. The function `f` will
    /// be called by Rocket just prior to launching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // A fairing that prints a message just before launching.
    /// let fairing = AdHoc::on_launch("Launch Count", |rocket| {
    ///     println!("Launching in T-3..2..1..");
    /// });
    /// ```
    pub fn on_launch<F: Send + 'static>(name: &'static str, f: F) -> AdHoc
        where F: FnOnce(&Cargo)
    {
        AdHoc { name, kind: AdHocKind::Launch(Mutex::new(Some(Box::new(f)))) }
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
        where F: for<'a> Fn(&'a mut Request<'_>, &'a Data) -> BoxFuture<'a, ()>
    {
        AdHoc { name, kind: AdHocKind::Request(Box::new(f)) }
    }
    // // FIXME: Can the generated future hold references to the request with this?
    // pub fn on_request<F, Fut>(name: &'static str, f: F) -> AdHoc
    // where
    //     F: for<'a> Fn(&'a mut Request<'_>, &'a Data) -> Fut + Send + Sync + 'static,
    //     Fut: Future<Output=()> + Send + 'static,
    // {
    //     AdHoc {
    //         name,
    //         kind: AdHocKind::Request(Box::new(|req, data| Box::pin(f(req, data))))
    //     }
    // }

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
        where F: for<'a> Fn(&'a Request<'_>, &'a mut Response<'_>) -> BoxFuture<'a, ()>
    {
        AdHoc { name, kind: AdHocKind::Response(Box::new(f)) }
    }
}

#[crate::async_trait]
impl Fairing for AdHoc {
    fn info(&self) -> Info {
        let kind = match self.kind {
            AdHocKind::Attach(_) => Kind::Attach,
            AdHocKind::Launch(_) => Kind::Launch,
            AdHocKind::Request(_) => Kind::Request,
            AdHocKind::Response(_) => Kind::Response,
        };

        Info { name: self.name, kind }
    }

    async fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        if let AdHocKind::Attach(ref mutex) = self.kind {
            let f = mutex.lock()
                .expect("AdHoc::Attach lock")
                .take()
                .expect("internal error: `on_attach` one-call invariant broken");
            f(rocket).await
        } else {
            Ok(rocket)
        }
    }

    fn on_launch(&self, state: &Cargo) {
        if let AdHocKind::Launch(ref mutex) = self.kind {
            let mut opt = mutex.lock().expect("AdHoc::Launch lock");
            let f = opt.take().expect("internal error: `on_launch` one-call invariant broken");
            f(state)
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, data: &Data) {
        if let AdHocKind::Request(ref callback) = self.kind {
            callback(req, data).await;
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        if let AdHocKind::Response(ref callback) = self.kind {
            callback(req, res).await;
        }
    }
}
