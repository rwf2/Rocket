use std::sync::Mutex;

use {Rocket, Request, Response, Data};
use fairing::{Fairing, Kind, Info};

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
///         req.set_method(Method::Put);
///     }));
/// ```
pub struct AdHoc {
    name: &'static str,
    kind: AdHocKind,
}

enum AdHocKind {
    /// An ad-hoc **attach** fairing. Called when the fairing is attached.
    Attach(Mutex<Option<Box<dyn FnOnce(Rocket) -> Result<Rocket, Rocket> + Send + 'static>>>),
    /// An ad-hoc **launch** fairing. Called just before Rocket launches.
    Launch(Mutex<Option<Box<dyn FnOnce(&Rocket) + Send + 'static>>>),
    /// An ad-hoc **request** fairing. Called when a request is received.
    Request(Box<dyn Fn(&mut Request, &Data) + Send + Sync + 'static>),
    /// An ad-hoc **response** fairing. Called when a response is ready to be
    /// sent to a client.
    Response(Box<dyn Fn(&Request, &mut Response) + Send + Sync + 'static>),
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
    /// let fairing = AdHoc::on_attach("No-Op", |rocket| Ok(rocket));
    /// ```
    pub fn on_attach<F>(name: &'static str, f: F) -> AdHoc
        where F: FnOnce(Rocket) -> Result<Rocket, Rocket> + Send + 'static
    {
        AdHoc { name, kind: AdHocKind::Attach(Mutex::new(Some(Box::new(f)))) }
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
    pub fn on_launch<F>(name: &'static str, f: F) -> AdHoc
        where F: FnOnce(&Rocket) + Send + 'static
    {
        AdHoc { name, kind: AdHocKind::Launch(Mutex::new(Some(Box::new(f)))) }
    }

    /// Constructs an `AdHoc` request fairing named `name`. The function `f`
    /// will be called by Rocket when a new request is received.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op request fairing.
    /// let fairing = AdHoc::on_request("Dummy", |req, data| {
    ///     // do something with the request and data...
    /// #   let (_, _) = (req, data);
    /// });
    /// ```
    pub fn on_request<F>(name: &'static str, f: F) -> AdHoc
        where F: Fn(&mut Request, &Data) + Send + Sync + 'static
    {
        AdHoc { name, kind: AdHocKind::Request(Box::new(f)) }
    }

    /// Constructs an `AdHoc` response fairing named `name`. The function `f`
    /// will be called by Rocket when a response is ready to be sent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::AdHoc;
    ///
    /// // The no-op response fairing.
    /// let fairing = AdHoc::on_response("Dummy", |req, resp| {
    ///     // do something with the request and pending response...
    /// #   let (_, _) = (req, resp);
    /// });
    /// ```
    pub fn on_response<F>(name: &'static str, f: F) -> AdHoc
        where F: Fn(&Request, &mut Response) + Send + Sync + 'static
    {
        AdHoc { name, kind: AdHocKind::Response(Box::new(f)) }
    }
}

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

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        if let AdHocKind::Attach(ref mutex) = self.kind {
            let mut opt = mutex.lock().expect("AdHoc::Attach lock");
            let f = opt.take().expect("internal error: `on_attach` one-call invariant broken");
            f(rocket)
        } else {
            Ok(rocket)
        }
    }

    fn on_launch(&self, rocket: &Rocket) {
        if let AdHocKind::Launch(ref mutex) = self.kind {
            let mut opt = mutex.lock().expect("AdHoc::Launch lock");
            let f = opt.take().expect("internal error: `on_launch` one-call invariant broken");
            f(rocket)
        }
    }

    fn on_request(&self, request: &mut Request, data: &Data) {
        if let AdHocKind::Request(ref callback) = self.kind {
            callback(request, data)
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if let AdHocKind::Response(ref callback) = self.kind {
            callback(request, response)
        }
    }
}
