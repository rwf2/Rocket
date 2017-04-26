//! Fairings: structured interposition at launch, request, and response time.
//!
//! Fairings allow for structured interposition at various points in the
//! application lifetime. Fairings can be seen as a restricted form of
//! "middleware". A fairing is simply a function with a particular signature
//! that Rocket will run at a requested point in a program. You can use fairings
//! to rewrite or record information about requests and responses, or to perform
//! an action once a Rocket application has launched.
//!
//! ## Attaching
//!
//! You must inform Rocket about fairings that you wish to be active by calling
//! the [`attach`](/rocket/struct.Rocket.html#method.attach) method on the
//! [`Rocket`](/rocket/struct.Rocket.html) instance and passing in the
//! appropriate [`Fairing`](/rocket/fairing/enum.Fairing.html). For instance, to
//! attach `Request` and `Response` fairings named `req_fairing` and
//! `res_fairing` to a new Rocket instance, you might write:
//!
//! ```rust
//! # use rocket::Fairing;
//! # let req_fairing = Fairing::Request(Box::new(|_, _| ()));
//! # let res_fairing = Fairing::Response(Box::new(|_, _| ()));
//! # #[allow(unused_variables)]
//! let rocket = rocket::ignite()
//!     .attach(vec![req_fairing, res_fairing]);
//! ```
//!
//! Once a fairing is attached, Rocket will execute it at the appropiate time,
//! which varies depending on the fairing type.

use {Rocket, Request, Response, Data};

// We might imagine that a request fairing returns an `Outcome`. If it returns
// `Success`, we don't do any routing and use that response directly. Same if it
// returns `Failure`. We only route if it returns `Forward`. I've chosen not to
// go this direction because I feel like request guards are the correct
// mechanism to use here. In other words, enabling this at the fairing level
// encourages implicit handling, a bad practice. Fairings can still, however,
// return a default `Response` if routing fails via a response fairing. For
// instance, to automatically handle preflight in CORS, a response fairing can
// check that the user didn't handle the `OPTIONS` request (404) and return an
// appropriate response. This allows the users to handle `OPTIONS` requests
// when they'd like but default to the fairing when they don't want to.

/// The type of a **launch** fairing callback.
///
/// The `Rocket` parameter is the `Rocket` instance being built. The launch
/// fairing can modify the `Rocket` instance arbitrarily.
///
/// TODO: Document fully with examples before 0.3.
pub type LaunchFn = Box<Fn(Rocket) -> Result<Rocket, Rocket> + Send + Sync + 'static>;
/// The type of a **request** fairing callback.
///
/// The `&mut Request` parameter is the incoming request, and the `&Data`
/// parameter is the incoming data in the request.
///
/// TODO: Document fully with examples before 0.3.
pub type RequestFn = Box<Fn(&mut Request, &Data) + Send + Sync + 'static>;
/// The type of a **response** fairing callback.
///
/// The `&Request` parameter is the request that was routed, and the `&mut
/// Response` parameter is the result response.
///
/// TODO: Document fully with examples before 0.3.
pub type ResponseFn = Box<Fn(&Request, &mut Response) + Send + Sync + 'static>;

/// An enum representing the three fairing types: launch, request, and response.
///
/// ## Fairing Types
///
/// The three types of fairings, launch, request, and response, operate as
/// follows:
///
///   * *Launch Fairings*
///
///     An attached launch fairing will be called immediately before the Rocket
///     application has launched. At this point, Rocket has opened a socket for
///     listening but has not yet begun accepting connections. A launch fairing
///     can arbitrarily modify the `Rocket` instance being launched. It returns
///     `Ok` if it would like launching to proceed nominally and `Err`
///     otherwise. If a launch fairing returns `Err`, launch is aborted. The
///     [`LaunchFn`](/rocket/fairing/type.LaunchFn.html) documentation contains
///     further information and tips on the function signature.
///
///   * *Request Fairings*
///
///     An attached request fairing is called when a request is received. At
///     this point, Rocket has parsed the incoming HTTP into a
///     [Request](/rocket/struct.Request.html) and
///     [Data](/rocket/struct.Data.html) object but has not routed the request.
///     A request fairing can modify the request at will and
///     [peek](/rocket/struct.Data.html#method.peek) into the incoming data. It
///     may not, however, abort or respond directly to the request; these issues
///     are better handled via [request
///     guards](/rocket/request/trait.FromRequest.html) or via response
///     fairings. A modified request is routed as if it was the original
///     request. The [`RequestFn`](/rocket/fairing/type.RequestFn.html)
///     documentation contains further information and tips on the function
///     signature.
///
///   * *Response Fairings*
///
///     An attached response fairing is called when a response is ready to be
///     sent to the client. At this point, Rocket has completed all routing,
///     including to error catchers, and has generated the would-be final
///     response. A response fairing can modify the response at will. A response
///     fairing, can, for example, provide a default response when the user
///     fails to handle the request by checking for 404 responses. The
///     [`ResponseFn`](/rocket/fairing/type.ResponseFn.html) documentation
///     contains further information and tips on the function signature.
///
/// See the [top-level documentation](/rocket/fairing/) for general information.
pub enum Fairing {
    /// A launch fairing. Called just before Rocket launches.
    Launch(LaunchFn),
    /// A request fairing. Called when a request is received.
    Request(RequestFn),
    /// A response fairing. Called when a response is ready to be sent.
    Response(ResponseFn),
}

#[derive(Default)]
pub(crate) struct Fairings {
    pub launch: Vec<LaunchFn>,
    pub request: Vec<RequestFn>,
    pub response: Vec<ResponseFn>,
}

impl Fairings {
    #[inline]
    pub fn new() -> Fairings {
        Fairings::default()
    }

    #[inline(always)]
    pub fn attach_all(&mut self, fairings: Vec<Fairing>) {
        for fairing in fairings {
            self.attach(fairing)
        }
    }

    #[inline]
    pub fn attach(&mut self, fairing: Fairing) {
        match fairing {
            Fairing::Launch(f) => self.launch.push(f),
            Fairing::Request(f) => self.request.push(f),
            Fairing::Response(f) => self.response.push(f),
        }
    }

    #[inline(always)]
    pub fn handle_launch(&mut self, mut rocket: Rocket) -> Option<Rocket> {
        let mut success = Some(());
        let launch_fairings = ::std::mem::replace(&mut self.launch, vec![]);
        for fairing in launch_fairings {
            rocket = fairing(rocket).unwrap_or_else(|r| { success = None; r });
        }

        success.map(|_| rocket)
    }

    #[inline(always)]
    pub fn handle_request(&self, req: &mut Request, data: &Data) {
        for fairing in &self.request {
            fairing(req, data);
        }
    }

    #[inline(always)]
    pub fn handle_response(&self, request: &Request, response: &mut Response) {
        for fairing in &self.response {
            fairing(request, response);
        }
    }

    fn num_attached(&self) -> usize {
        self.launch.len() + self.request.len() + self.response.len()
    }

    pub fn pretty_print_counts(&self) {
        use term_painter::ToStyle;
        use term_painter::Color::{White, Magenta};

        if self.num_attached() > 0 {
            info!("📡  {}:", Magenta.paint("Fairings"));
        }

        if !self.launch.is_empty() {
            info_!("{} launch", White.paint(self.launch.len()));
        }

        if !self.request.is_empty() {
            info_!("{} request", White.paint(self.request.len()));
        }

        if !self.response.is_empty() {
            info_!("{} response", White.paint(self.response.len()));
        }
    }
}
