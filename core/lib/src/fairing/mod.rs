//! Fairings: callbacks at launch, liftoff, request, and response time.
//!
//! Fairings allow for structured interposition at various points in the
//! application lifetime. Fairings can be seen as a restricted form of
//! "middleware". A fairing is an arbitrary structure with methods representing
//! callbacks that Rocket will run at requested points in a program. You can use
//! fairings to rewrite or record information about requests and responses, or
//! to perform an action once a Rocket application has launched.
//!
//! To learn more about writing a fairing, see the [`Fairing`] trait
//! documentation. You can also use [`AdHoc`] to create a fairing on-the-fly
//! from a closure or function.
//!
//! ## Attaching
//!
//! You must inform Rocket about fairings that you wish to be active by calling
//! [`Rocket::attach()`] method on the application's [`Rocket`] instance and
//! passing in the appropriate [`Fairing`]. For instance, to attach fairings
//! named `req_fairing` and `res_fairing` to a new Rocket instance, you might
//! write:
//!
//! ```rust
//! # use rocket::fairing::AdHoc;
//! # let req_fairing = AdHoc::on_request("Request", |_, _| Box::pin(async move {}));
//! # let res_fairing = AdHoc::on_response("Response", |_, _| Box::pin(async move {}));
//! let rocket = rocket::build()
//!     .attach(req_fairing)
//!     .attach(res_fairing);
//! ```
//!
//! Once a fairing is attached, Rocket will execute it at the appropriate time,
//! which varies depending on the fairing implementation. See the [`Fairing`]
//! trait documentation for more information on the dispatching of fairing
//! methods.
//!
//! [`Fairing`]: crate::fairing::Fairing
//!
//! ## Ordering
//!
//! `Fairing`s are executed in the order in which they are attached: the first
//! attached fairing has its callbacks executed before all others. A fairing can
//! be attached any number of times. Except for [singleton
//! fairings](Fairing#singletons), all attached instances are polled at runtime.
//! Fairing callbacks may not be commutative; the order in which fairings are
//! attached may be significant. It is thus important to communicate specific
//! fairing functionality clearly.
//!
//! Furthermore, a `Fairing` should take care to act locally so that the actions
//! of other `Fairings` are not jeopardized. For instance, unless it is made
//! abundantly clear, a fairing should not rewrite every request.

use std::any::Any;

use crate::{Rocket, Request, Response, Data, Build, Orbit};

mod fairings;
mod ad_hoc;
mod info_kind;

pub(crate) use self::fairings::Fairings;
pub use self::ad_hoc::AdHoc;
pub use self::info_kind::{Info, Kind};

/// A type alias for the return `Result` type of [`Fairing::on_ignite()`].
pub type Result<T = Rocket<Build>, E = Rocket<Build>> = std::result::Result<T, E>;

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

/// Trait implemented by fairings: Rocket's structured middleware.
///
/// # Considerations
///
/// Fairings are a large hammer that can easily be abused and misused. If you
/// are considering writing a `Fairing` implementation, first consider if it is
/// appropriate to do so. While middleware is often the best solution to some
/// problems in other frameworks, it is often a suboptimal solution in Rocket.
/// This is because Rocket provides richer mechanisms such as [request guards]
/// and [data guards] that can be used to accomplish the same objective in a
/// cleaner, more composable, and more robust manner.
///
/// As a general rule of thumb, only _globally applicable actions_ should be
/// implemented via fairings. For instance, you should _not_ use a fairing to
/// implement authentication or authorization (preferring to use a [request
/// guard] instead) _unless_ the authentication or authorization applies to the
/// entire application. On the other hand, you _should_ use a fairing to record
/// timing and/or usage statistics or to implement global security policies.
///
/// [request guard]: crate::request::FromRequest
/// [request guards]: crate::request::FromRequest
/// [data guards]: crate::data::FromData
///
/// ## Fairing Callbacks
///
/// There are four kinds of fairing callbacks: launch, liftoff, request, and
/// response. A fairing can request any combination of these callbacks through
/// the `kind` field of the [`Info`] structure returned from the `info` method.
/// Rocket will only invoke the callbacks identified in the fairing's [`Kind`].
///
/// The four callback kinds are as follows:
///
///   * **Ignite (`on_ignite`)**
///
///     An ignite callback, represented by the [`Fairing::on_ignite()`] method,
///     is called just prior to liftoff, during ignition. The state of the
///     `Rocket` instance is, at this point, not finalized, as it may be
///     modified at will by other ignite fairings.
///
///     All ignite callbacks are executed in breadth-first `attach()` order. A
///     callback `B` executing after a callback `A` can view changes made by `A`
///     but not vice-versa.
///
///     An ignite callback can arbitrarily modify the `Rocket` instance being
///     constructed. It should take care not to introduce infinite recursion by
///     recursively attaching ignite fairings. It returns `Ok` if it would like
///     ignition and launch to proceed nominally and `Err` otherwise. If an
///     ignite fairing returns `Err`, launch will be aborted. All ignite
///     fairings are executed even if one or more signal a failure.
///
///   * **Liftoff (`on_liftoff`)**
///
///     A liftoff callback, represented by the [`Fairing::on_liftoff()`] method,
///     is called immediately after a Rocket application has launched. At this
///     point, Rocket has opened a socket for listening but has not yet begun
///     accepting connections. A liftoff callback can inspect the `Rocket`
///     instance that has launched but not otherwise gracefully abort launch.
///
///   * **Request (`on_request`)**
///
///     A request callback, represented by the [`Fairing::on_request()`] method,
///     is called just after a request is received, immediately after
///     pre-processing the request with method changes due to `_method` form
///     fields. At this point, Rocket has parsed the incoming HTTP request into
///     [`Request`] and [`Data`] structures but has not routed the request. A
///     request callback can modify the request at will and [`Data::peek()`]
///     into the incoming data. It may not, however, abort or respond directly
///     to the request; these issues are better handled via [request guards] or
///     via response callbacks. Any modifications to a request are persisted and
///     can potentially alter how a request is routed.
///
///   * **Response (`on_response`)**
///
///     A response callback, represented by the [`Fairing::on_response()`]
///     method, is called when a response is ready to be sent to the client. At
///     this point, Rocket has completed all routing, including to error
///     catchers, and has generated the would-be final response. A response
///     callback can modify the response at will. For example, a response
///     callback can provide a default response when the user fails to handle
///     the request by checking for 404 responses. Note that a given `Request`
///     may have changed between `on_request` and `on_response` invocations.
///     Apart from any change made by other fairings, Rocket sets the method for
///     `HEAD` requests to `GET` if there is no matching `HEAD` handler for that
///     request. Additionally, Rocket will automatically strip the body for
///     `HEAD` requests _after_ response fairings have run.
///
/// # Singletons
///
/// In general, any number of instances of a given fairing type can be attached
/// to one instance of `Rocket`. If this is not desired, a fairing can request
/// to be a singleton by specifying [`Kind::Singleton`]. Only the _last_
/// attached instance of a singleton will be preserved at ignite-time. That is,
/// an attached singleton instance will replace any previously attached
/// instance. The [`Shield`](crate::shield::Shield) fairing is an example of a
/// singleton fairing.
///
/// # Implementing
///
/// A `Fairing` implementation has one required method: [`info`]. A `Fairing`
/// can also implement any of the available callbacks: `on_ignite`, `on_liftoff`,
/// `on_request`, and `on_response`. A `Fairing` _must_ set the appropriate
/// callback kind in the `kind` field of the returned `Info` structure from
/// [`info`] for a callback to actually be called by Rocket.
///
/// ## Fairing `Info`
///
/// Every `Fairing` must implement the [`info`] method, which returns an
/// [`Info`] structure. This structure is used by Rocket to:
///
///   1. Assign a name to the `Fairing`.
///
///      This is the `name` field, which can be any arbitrary string. Name your
///      fairing something illustrative. The name will be logged during the
///      application's ignition procedures.
///
///   2. Determine which callbacks to actually issue on the `Fairing`.
///
///      This is the `kind` field of type [`Kind`]. This field is a bitset that
///      represents the kinds of callbacks the fairing wishes to receive. Rocket
///      will only invoke the callbacks that are flagged in this set. `Kind`
///      structures can be `or`d together to represent any combination of kinds
///      of callbacks. For instance, to request liftoff and response callbacks,
///      return a `kind` field with the value `Kind::Liftoff | Kind::Response`.
///
/// [`info`]: Fairing::info()
///
/// ## Restrictions
///
/// A `Fairing` must be [`Send`] + [`Sync`] + `'static`. This means that the
/// fairing must be sendable across thread boundaries (`Send`), thread-safe
/// (`Sync`), and have only `'static` references, if any (`'static`). Note that
/// these bounds _do not_ prohibit a `Fairing` from holding state: the state
/// need simply be thread-safe and statically available or heap allocated.
///
/// ## Async Trait
///
/// [`Fairing`] is an _async_ trait. Implementations of `Fairing` must be
/// decorated with an attribute of `#[rocket::async_trait]`:
///
/// ```rust
/// use rocket::{Rocket, Request, Data, Response, Build, Orbit};
/// use rocket::fairing::{self, Fairing, Info, Kind};
///
/// # struct MyType;
/// #[rocket::async_trait]
/// impl Fairing for MyType {
///     fn info(&self) -> Info {
///         /* ... */
///         # unimplemented!()
///     }
///
///     async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
///         /* ... */
///         # unimplemented!()
///     }
///
///     async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
///         /* ... */
///         # unimplemented!()
///     }
///
///     async fn on_request(&self, req: &mut Request<'_>, data: &mut Data<'_>) {
///         /* ... */
///         # unimplemented!()
///     }
///
///     async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
///         /* ... */
///         # unimplemented!()
///     }
/// }
/// ```
///
/// ## Example
///
/// As an example, we want to record the number of `GET` and `POST` requests
/// that our application has received. While we could do this with [request
/// guards] and [managed state](crate::State), it would require us to annotate
/// every `GET` and `POST` request with custom types, polluting handler
/// signatures. Instead, we can create a simple fairing that acts globally.
///
/// The `Counter` fairing below records the number of all `GET` and `POST`
/// requests received. It makes these counts available at a special `'/counts'`
/// path.
///
/// ```rust
/// use std::future::Future;
/// use std::io::Cursor;
/// use std::pin::Pin;
/// use std::sync::atomic::{AtomicUsize, Ordering};
///
/// use rocket::{Request, Data, Response};
/// use rocket::fairing::{Fairing, Info, Kind};
/// use rocket::http::{Method, ContentType, Status};
///
/// #[derive(Default)]
/// struct Counter {
///     get: AtomicUsize,
///     post: AtomicUsize,
/// }
///
/// #[rocket::async_trait]
/// impl Fairing for Counter {
///     fn info(&self) -> Info {
///         Info {
///             name: "GET/POST Counter",
///             kind: Kind::Request | Kind::Response
///         }
///     }
///
///     async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
///         if req.method() == Method::Get {
///             self.get.fetch_add(1, Ordering::Relaxed);
///         } else if req.method() == Method::Post {
///             self.post.fetch_add(1, Ordering::Relaxed);
///         }
///     }
///
///     async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
///         // Don't change a successful user's response, ever.
///         if res.status() != Status::NotFound {
///             return
///         }
///
///         if req.method() == Method::Get && req.uri().path() == "/counts" {
///             let get_count = self.get.load(Ordering::Relaxed);
///             let post_count = self.post.load(Ordering::Relaxed);
///
///             let body = format!("Get: {}\nPost: {}", get_count, post_count);
///             res.set_status(Status::Ok);
///             res.set_header(ContentType::Plain);
///             res.set_sized_body(body.len(), Cursor::new(body));
///         }
///     }
/// }
/// ```
///
/// ## Request-Local State
///
/// Fairings can use [request-local state] to persist or carry data between
/// requests and responses, or to pass data to a request guard.
///
/// As an example, the following fairing uses request-local state to time
/// requests, setting an `X-Response-Time` header on all responses with the
/// elapsed time. It also exposes the start time of a request via a `StartTime`
/// request guard.
///
/// ```rust
/// # use std::future::Future;
/// # use std::pin::Pin;
/// # use std::time::{Duration, SystemTime};
/// # use rocket::{Request, Data, Response};
/// # use rocket::fairing::{Fairing, Info, Kind};
/// # use rocket::http::Status;
/// # use rocket::request::{self, FromRequest};
/// #
/// /// Fairing for timing requests.
/// pub struct RequestTimer;
///
/// /// Value stored in request-local state.
/// #[derive(Copy, Clone)]
/// struct TimerStart(Option<SystemTime>);
///
/// #[rocket::async_trait]
/// impl Fairing for RequestTimer {
///     fn info(&self) -> Info {
///         Info {
///             name: "Request Timer",
///             kind: Kind::Request | Kind::Response
///         }
///     }
///
///     /// Stores the start time of the request in request-local state.
///     async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
///         // Store a `TimerStart` instead of directly storing a `SystemTime`
///         // to ensure that this usage doesn't conflict with anything else
///         // that might store a `SystemTime` in request-local cache.
///         request.local_cache(|| TimerStart(Some(SystemTime::now())));
///     }
///
///     /// Adds a header to the response indicating how long the server took to
///     /// process the request.
///     async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
///         let start_time = req.local_cache(|| TimerStart(None));
///         if let Some(Ok(duration)) = start_time.0.map(|st| st.elapsed()) {
///             let ms = duration.as_secs() * 1000 + duration.subsec_millis() as u64;
///             res.set_raw_header("X-Response-Time", format!("{} ms", ms));
///         }
///     }
/// }
///
/// /// Request guard used to retrieve the start time of a request.
/// #[derive(Copy, Clone)]
/// pub struct StartTime(pub SystemTime);
///
/// // Allows a route to access the time a request was initiated.
/// #[rocket::async_trait]
/// impl<'r> FromRequest<'r> for StartTime {
///     type Error = ();
///
///     async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
///         match *request.local_cache(|| TimerStart(None)) {
///             TimerStart(Some(time)) => request::Outcome::Success(StartTime(time)),
///             TimerStart(None) => request::Outcome::Failure((Status::InternalServerError, ())),
///         }
///     }
/// }
/// ```
///
/// [request-local state]: https://rocket.rs/v0.5-rc/guide/state/#request-local-state
#[crate::async_trait]
pub trait Fairing: Send + Sync + Any + 'static {
    /// Returns an [`Info`] structure containing the `name` and [`Kind`] of this
    /// fairing. The `name` can be any arbitrary string. `Kind` must be an `or`d
    /// set of `Kind` variants.
    ///
    /// This is the only required method of a `Fairing`. All other methods have
    /// no-op default implementations.
    ///
    /// Rocket will only dispatch callbacks to this fairing for the kinds in the
    /// `kind` field of the returned `Info` structure. For instance, if
    /// `Kind::Ignite | Kind::Request` is used, then Rocket will only call the
    /// `on_ignite` and `on_request` methods of the fairing. Similarly, if
    /// `Kind::Response` is used, Rocket will only call the `on_response` method
    /// of this fairing.
    ///
    /// # Example
    ///
    /// An `info` implementation for `MyFairing`: a fairing named "My Custom
    /// Fairing" that is both an ignite and response fairing.
    ///
    /// ```rust
    /// use rocket::fairing::{Fairing, Info, Kind};
    ///
    /// struct MyFairing;
    ///
    /// impl Fairing for MyFairing {
    ///     fn info(&self) -> Info {
    ///         Info {
    ///             name: "My Custom Fairing",
    ///             kind: Kind::Ignite | Kind::Response
    ///         }
    ///     }
    /// }
    /// ```
    fn info(&self) -> Info;

    /// The ignite callback. Returns `Ok` if ignition should proceed and `Err`
    /// if ignition and launch should be aborted.
    ///
    /// This method is called during ignition and if `Kind::Ignite` is in the
    /// `kind` field of the `Info` structure for this fairing. The `rocket`
    /// parameter is the `Rocket` instance that is currently being built for
    /// this application.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation of this method simply returns `Ok(rocket)`.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result { Ok(rocket) }

    /// The liftoff callback.
    ///
    /// This method is called just after launching the application if
    /// `Kind::Liftoff` is in the `kind` field of the `Info` structure for this
    /// fairing. The `Rocket` parameter corresponds to the lauched application.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation of this method does nothing.
    async fn on_liftoff(&self, _rocket: &Rocket<Orbit>) { }

    /// The request callback.
    ///
    /// This method is called when a new request is received if `Kind::Request`
    /// is in the `kind` field of the `Info` structure for this fairing. The
    /// `&mut Request` parameter is the incoming request, and the `&Data`
    /// parameter is the incoming data in the request.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation of this method does nothing.
    async fn on_request(&self, _req: &mut Request<'_>, _data: &mut Data<'_>) {}

    /// The response callback.
    ///
    /// This method is called when a response is ready to be issued to a client
    /// if `Kind::Response` is in the `kind` field of the `Info` structure for
    /// this fairing. The `&Request` parameter is the request that was routed,
    /// and the `&mut Response` parameter is the resulting response.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation of this method does nothing.
    async fn on_response<'r>(&self, _req: &'r Request<'_>, _res: &mut Response<'r>) {}
}

#[crate::async_trait]
impl<T: Fairing> Fairing for std::sync::Arc<T> {
    #[inline]
    fn info(&self) -> Info {
        (self as &T).info()
    }

    #[inline]
    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result {
        (self as &T).on_ignite(rocket).await
    }

    #[inline]
    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        (self as &T).on_liftoff(rocket).await
    }

    #[inline]
    async fn on_request(&self, req: &mut Request<'_>, data: &mut Data<'_>) {
        (self as &T).on_request(req, data).await
    }

    #[inline]
    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        (self as &T).on_response(req, res).await
    }
}
