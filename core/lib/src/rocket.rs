use std::collections::HashMap;
use std::convert::From;
use std::str::{from_utf8, FromStr};
use std::cmp::min;
use std::io::{self, Write};
use std::time::Duration;
use std::mem;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use futures::{Future, Stream};
use futures::future::{self, FutureResult};

use yansi::Paint;
use state::Container;
use tokio::net::TcpListener;
use tokio::prelude::{Future as _, Stream as _};

#[cfg(feature = "tls")] use crate::http::tls::TlsAcceptor;

use crate::{logger, handler};
use crate::ext::ReadExt;
use crate::config::{self, Config, LoggedValue};
use crate::request::{Request, FormItems};
use crate::data::Data;
use crate::response::{Body, Response};
use crate::router::{Router, Route};
use crate::catcher::{self, Catcher};
use crate::outcome::Outcome;
use crate::error::{LaunchError, LaunchErrorKind};
use crate::fairing::{Fairing, Fairings};

use crate::http::{Method, Status, Header};
use crate::http::hyper::{self, header};
use crate::http::uri::Origin;

/// The main `Rocket` type: used to mount routes and catchers and launch the
/// application.
pub struct Rocket {
    crate config: Config,
    router: Router,
    default_catchers: HashMap<u16, Catcher>,
    catchers: HashMap<u16, Catcher>,
    crate state: Container,
    fairings: Fairings,
}

struct RocketArcs {
    config: Arc<Config>,
    router: Arc<Router>,
    default_catchers: Arc<HashMap<u16, Catcher>>,
    catchers: Arc<HashMap<u16, Catcher>>,
    state: Arc<Container>,
    fairings: Arc<Fairings>,
}

impl<Ctx> hyper::MakeService<Ctx> for RocketArcs {
    type ReqBody = hyper::Body;
    type ResBody = hyper::Body;
    type Error = hyper::Error;
    type Service = RocketHyperService;
    type Future = FutureResult<Self::Service, Self::MakeError>;
    type MakeError = Self::Error;

    fn make_service(&mut self, _: Ctx) -> Self::Future {
        future::ok(RocketHyperService::new(self))
    }
}

#[derive(Clone)]
pub struct RocketHyperService {
    config: Arc<Config>,
    router: Arc<Router>,
    default_catchers: Arc<HashMap<u16, Catcher>>,
    catchers: Arc<HashMap<u16, Catcher>>,
    state: Arc<Container>,
    fairings: Arc<Fairings>,
}

#[doc(hidden)]
impl hyper::Service for RocketHyperService {
    type ReqBody = hyper::Body;
    type ResBody = hyper::Body;
    type Error = hyper::Error;
    //type Future = FutureResult<hyper::Response<Self::ResBody>, Self::Error>;
    type Future = Box<future::Future<Item = hyper::Response<Self::ResBody>, Error = Self::Error> + Send>;

    // This function tries to hide all of the Hyper-ness from Rocket. It
    // essentially converts Hyper types into Rocket types, then calls the
    // `dispatch` function, which knows nothing about Hyper. Because responding
    // depends on the `HyperResponse` type, this function does the actual
    // response processing.
    fn call<'h>(
        &mut self,
        hyp_req: hyper::Request<Self::ReqBody>,
    ) -> Self::Future {
        let (parts, body) = hyp_req.into_parts();

        // Convert the Hyper request into a Rocket request.
        let req_res = Request::from_hyp(&self.config, &self.state, &parts);
        let mut req = match req_res {
            Ok(req) => req,
            Err(e) => {
                error!("Bad incoming request: {}", e);
                // TODO: We don't have a request to pass in, so we just
                // fabricate one. This is weird. We should let the user know
                // that we failed to parse a request (by invoking some special
                // handler) instead of doing this.
                let dummy = Request::new(&self.config, &self.state, Method::Get, Origin::dummy());
                let r = self.handle_error(Status::BadRequest, &dummy);
                return Box::new(future::ok(hyper::Response::from(r)));
            }
        };

        let this = self.clone();

        let response = body.concat2()
            .map(move |chunk| {
                let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
                let data = Data::new(body);

                // TODO: Due to life time constraints the clone of the service has been made.
                // TODO: It should not be necessary but it is required to find a better solution
                let mut req = Request::from_hyp(&this.config, &this.state, &parts).unwrap();
                // Dispatch the request to get a response, then write that response out.
                let response = this.dispatch(&mut req, data);
                hyper::Response::from(response)
            });

        Box::new(response)
    }
}

impl RocketHyperService {

    #[inline]
    fn new(rocket: &RocketArcs) -> RocketHyperService {
        RocketHyperService {
            config: rocket.config.clone(),
            router: rocket.router.clone(),
            default_catchers: rocket.default_catchers.clone(),
            catchers: rocket.catchers.clone(),
            state: rocket.state.clone(),
            fairings: rocket.fairings.clone(),
        }
    }

    /// Preprocess the request for Rocket things. Currently, this means:
    ///
    ///   * Rewriting the method in the request if _method form field exists.
    ///
    /// Keep this in-sync with derive_form when preprocessing form fields.
    fn preprocess_request(&self, req: &mut Request<'_>, data: &Data) {
        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        let data_len = data.peek().len();
        let (min_len, max_len) = ("_method=get".len(), "_method=delete".len());
        let is_form = req.content_type().map_or(false, |ct| ct.is_form());

        if is_form && req.method() == Method::Post && data_len >= min_len {
            if let Ok(form) = from_utf8(&data.peek()[..min(data_len, max_len)]) {
                let method: Option<Result<Method, _>> = FormItems::from(form)
                    .filter(|item| item.key.as_str() == "_method")
                    .map(|item| item.value.parse())
                    .next();

                if let Some(Ok(method)) = method {
                    req.set_method(method);
                }
            }
        }
    }

    #[inline]
    crate fn dispatch<'s, 'r>(
        &'s self,
        request: &'r mut Request<'s>,
        data: Data
    ) -> Response<'r> {
        info!("{}:", request);

        // Do a bit of preprocessing before routing.
        self.preprocess_request(request, &data);

        // Run the request fairings.
        self.fairings.handle_request(request, &data);

        // Remember if the request is a `HEAD` request for later body stripping.
        let was_head_request = request.method() == Method::Head;

        // Route the request and run the user's handlers.
        let mut response = self.route_and_process(request, data);

        // Add a default 'Server' header if it isn't already there.
        // TODO: If removing Hyper, write out `Date` header too.
        if !response.headers().contains("Server") {
            response.set_header(Header::new("Server", "Rocket"));
        }

        // Run the response fairings.
        self.fairings.handle_response(request, &mut response);

        // Strip the body if this is a `HEAD` request.
        if was_head_request {
            response.strip_body();
        }

        response
    }

    /// Route the request and process the outcome to eventually get a response.
    fn route_and_process<'s, 'r>(
        &'s self,
        request: &'r Request<'s>,
        data: Data
    ) -> Response<'r> {
        match self.route(request, data) {
            Outcome::Success(mut response) => {
                // A user's route responded! Set the cookies.
                for cookie in request.cookies().delta() {
                    response.adjoin_header(cookie);
                }

                response
            }
            Outcome::Forward(data) => {
                // There was no matching route. Autohandle `HEAD` requests.
                if request.method() == Method::Head {
                    info_!("Autohandling {} request.", Paint::default("HEAD").bold());

                    // Dispatch the request again with Method `GET`.
                    request._set_method(Method::Get);
                    self.route_and_process(request, data)
                } else {
                    // No match was found and it can't be autohandled. 404.
                    self.handle_error(Status::NotFound, request)
                }
            }
            Outcome::Failure(status) => self.handle_error(status, request)
        }
    }

    /// Tries to find a `Responder` for a given `request`. It does this by
    /// routing the request and calling the handler for each matching route
    /// until one of the handlers returns success or failure, or there are no
    /// additional routes to try (forward). The corresponding outcome for each
    /// condition is returned.
    //
    // TODO: We _should_ be able to take an `&mut` here and mutate the request
    // at any pointer _before_ we pass it to a handler as long as we drop the
    // outcome. That should be safe. Since no mutable borrow can be held
    // (ensuring `handler` takes an immutable borrow), any caller to `route`
    // should be able to supply an `&mut` and retain an `&` after the call.
    #[inline]
    crate fn route<'s, 'r>(
        &'s self,
        request: &'r Request<'s>,
        mut data: Data,
    ) -> handler::Outcome<'r> {
        // Go through the list of matching routes until we fail or succeed.
        let matches = self.router.route(request);
        for route in matches {
            // Retrieve and set the requests parameters.
            info_!("Matched: {}", route);
            request.set_route(route);

            // Dispatch the request to the handler.
            let outcome = route.handler.handle(request, data);

            // Check if the request processing completed or if the request needs
            // to be forwarded. If it does, continue the loop to try again.
            info_!("{} {}", Paint::default("Outcome:").bold(), outcome);
            match outcome {
                o@Outcome::Success(_) | o@Outcome::Failure(_) => return o,
                Outcome::Forward(unused_data) => data = unused_data,
            };
        }

        error_!("No matching routes for {}.", request);
        Outcome::Forward(data)
    }

    // Finds the error catcher for the status `status` and executes it for the
    // given request `req`. If a user has registered a catcher for `status`, the
    // catcher is called. If the catcher fails to return a good response, the
    // 500 catcher is executed. If there is no registered catcher for `status`,
    // the default catcher is used.
    crate fn handle_error<'r>(
        &self,
        status: Status,
        req: &'r Request<'_>
    ) -> Response<'r> {
        warn_!("Responding with {} catcher.", Paint::red(&status));

        // Try to get the active catcher but fallback to user's 500 catcher.
        let catcher = self.catchers.get(&status.code).unwrap_or_else(|| {
            error_!("No catcher found for {}. Using 500 catcher.", status);
            self.catchers.get(&500).expect("500 catcher.")
        });

        // Dispatch to the user's catcher. If it fails, use the default 500.
        catcher.handle(req).unwrap_or_else(|err_status| {
            error_!("Catcher failed with status: {}!", err_status);
            warn_!("Using default 500 error catcher.");
            let default = self.default_catchers.get(&500).expect("Default 500");
            default.handle(req).expect("Default 500 response.")
        })
    }
}

impl Rocket {

    #[inline]
    crate fn dispatch<'s, 'r>(
        &'s self,
        request: &'r mut Request<'s>,
        data: Data
    ) -> Response<'r> {
        unimplemented!("TODO")
    }

    crate fn handle_error<'r>(
        &self,
        status: Status,
        req: &'r Request
    ) -> Response<'r> {
        unimplemented!("TODO")
    }

    /// Create a new `Rocket` application using the configuration information in
    /// `Rocket.toml`. If the file does not exist or if there is an I/O error
    /// reading the file, the defaults are used. See the [`config`]
    /// documentation for more information on defaults.
    ///
    /// This method is typically called through the
    /// [`rocket::ignite()`](crate::ignite) alias.
    ///
    /// # Panics
    ///
    /// If there is an error parsing the `Rocket.toml` file, this functions
    /// prints a nice error message and then exits the process.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # {
    /// rocket::ignite()
    /// # };
    /// ```
    #[inline]
    pub fn ignite() -> Rocket {
        // Note: init() will exit the process under config errors.
        Rocket::configured(config::init())
    }

    /// Creates a new `Rocket` application using the supplied custom
    /// configuration. The `Rocket.toml` file, if present, is ignored. Any
    /// environment variables setting config parameters are ignored.
    ///
    /// This method is typically called through the `rocket::custom` alias.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    /// # use rocket::config::ConfigError;
    ///
    /// # #[allow(dead_code)]
    /// # fn try_config() -> Result<(), ConfigError> {
    /// let config = Config::build(Environment::Staging)
    ///     .address("1.2.3.4")
    ///     .port(9234)
    ///     .finalize()?;
    ///
    /// # #[allow(unused_variables)]
    /// let app = rocket::custom(config);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn custom(config: Config) -> Rocket {
        Rocket::configured(config)
    }

    #[inline]
    fn configured(config: Config) -> Rocket {
        if logger::try_init(config.log_level, false) {
            // Temporary weaken log level for launch info.
            logger::push_max_level(logger::LoggingLevel::Normal);
        }

        launch_info!("{}Configured for {}.", Paint::masked("🔧 "), config.environment);
        launch_info_!("address: {}", Paint::default(&config.address).bold());
        launch_info_!("port: {}", Paint::default(&config.port).bold());
        launch_info_!("log: {}", Paint::default(config.log_level).bold());
        launch_info_!("workers: {}", Paint::default(config.workers).bold());
        launch_info_!("secret key: {}", Paint::default(&config.secret_key).bold());
        launch_info_!("limits: {}", Paint::default(&config.limits).bold());

        match config.keep_alive {
            Some(v) => launch_info_!("keep-alive: {}", Paint::default(format!("{}s", v)).bold()),
            None => launch_info_!("keep-alive: {}", Paint::default("disabled").bold()),
        }

        let tls_configured = config.tls.is_some();
        if tls_configured && cfg!(feature = "tls") {
            launch_info_!("tls: {}", Paint::default("enabled").bold());
        } else if tls_configured {
            error_!("tls: {}", Paint::default("disabled").bold());
            error_!("tls is configured, but the tls feature is disabled");
        } else {
            launch_info_!("tls: {}", Paint::default("disabled").bold());
        }

        if config.secret_key.is_generated() && config.environment.is_prod() {
            warn!("environment is 'production', but no `secret_key` is configured");
        }

        for (name, value) in config.extras() {
            launch_info_!("{} {}: {}",
                          Paint::yellow("[extra]"), name,
                          Paint::default(LoggedValue(value)).bold());
        }

        Rocket {
            config,
            router: Router::new(),
            default_catchers: catcher::defaults::get(),
            catchers: catcher::defaults::get(),
            state: Container::new(),
            fairings: Fairings::new(),
        }
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
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
    /// # #[macro_use] extern crate rocket;
    /// #
    /// #[get("/world")]
    /// fn hi() -> &'static str {
    ///     "Hello!"
    /// }
    ///
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite().mount("/hello", routes![hi])
    /// #       .launch();
    /// # }
    /// }
    /// ```
    ///
    /// Manually create a route named `hi` at path `"/world"` mounted at base
    /// `"/hello"`. Requests to the `/hello/world` URI will be dispatched to the
    /// `hi` route.
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method::*;
    ///
    /// fn hi<'r>(req: &'r Request, _: Data) -> Outcome<'r> {
    ///     Outcome::from(req, "Hello!")
    /// }
    ///
    /// # if false { // We don't actually want to launch the server in an example.
    /// rocket::ignite().mount("/hello", vec![Route::new(Get, "/world", hi)])
    /// #     .launch();
    /// # }
    /// ```
    #[inline]
    pub fn mount<R: Into<Vec<Route>>>(mut self, base: &str, routes: R) -> Self {
        info!("{}{} {}{}",
              Paint::masked("🛰  "),
              Paint::magenta("Mounting"),
              Paint::blue(base),
              Paint::magenta(":"));

        let base_uri = Origin::parse(base)
            .unwrap_or_else(|e| {
                error_!("Invalid origin URI '{}' used as mount point.", base);
                panic!("Error: {}", e);
            });

        if base_uri.query().is_some() {
            error_!("Mount point '{}' contains query string.", base);
            panic!("Invalid mount point.");
        }

        let mut router = self.router.clone();
        for mut route in routes.into() {
            let path = route.uri.clone();
            if let Err(e) = route.set_uri(base_uri.clone(), path) {
                error_!("{}", e);
                panic!("Invalid route URI.");
            }

            info_!("{}", route);
            router.add(route);
        }

        self.router = router;

        self
    }

    /// Registers all of the catchers in the supplied vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
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
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .register(catchers![internal_error, not_found])
    /// #       .launch();
    /// # }
    /// }
    /// ```
    #[inline]
    pub fn register(mut self, catchers: Vec<Catcher>) -> Self {
        info!("{}{}", Paint::masked("👾 "), Paint::magenta("Catchers:"));

        let mut current_catchers = self.catchers.clone();

        for c in catchers {
            if self.catchers.get(&c.code).map_or(false, |e| !e.is_default) {
                info_!("{} {}", c, Paint::yellow("(warning: duplicate catcher!)"));
            } else {
                info_!("{}", c);
            }

            current_catchers.insert(c.code, c);
        }

        self.catchers = current_catchers;

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
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
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
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .mount("/", routes![index])
    ///         .manage(MyValue(10))
    ///         .launch();
    /// # }
    /// }
    /// ```
    #[inline]
    pub fn manage<T: Send + Sync + 'static>(self, state: T) -> Self {
        if !self.state.set::<T>(state) {
            error!("State for this type is already being managed!");
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
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Launch Message", |_| {
    ///             println!("Rocket is launching!");
    ///         }))
    ///         .launch();
    /// # }
    /// }
    /// ```
    #[inline]
    pub fn attach<F: Fairing>(mut self, fairing: F) -> Self {
        // Attach (and run attach) fairings, which requires us to move `self`.
        let mut fairings = mem::replace(&mut self.fairings, Fairings::new());
        self = fairings.attach(Box::new(fairing), self);

        // Make sure we keep all fairings around: the old and newly added ones!
        fairings.append(self.fairings);
        self.fairings = fairings;
        self
    }

    crate fn prelaunch_check(mut self) -> Result<Rocket, LaunchError> {
        self.router = match self.router.collisions() {
            Ok(router) => router,
            Err(e) => return Err(LaunchError::new(LaunchErrorKind::Collision(e)))
        };

        if let Some(failures) = self.fairings.failures() {
            return Err(LaunchError::new(LaunchErrorKind::FailedFairings(failures.to_vec())))
        }

        Ok(self)
    }

    /// Starts the application server and begins listening for and dispatching
    /// requests to mounted routes and catchers. Unless there is an error, this
    /// function does not return and blocks until program termination.
    ///
    /// # Error
    ///
    /// If there is a problem starting the application, a [`LaunchError`] is
    /// returned. Note that a value of type `LaunchError` panics if dropped
    /// without first being inspected. See the [`LaunchError`] documentation for
    /// more information.
    ///
    /// # Example
    ///
    /// ```rust
    /// # if false {
    /// rocket::ignite().launch();
    /// # }
    /// ```
    pub fn launch(mut self) -> LaunchError {
        self = match self.prelaunch_check() {
            Ok(rocket) => rocket,
            Err(launch_error) => return launch_error
        };

        self.fairings.pretty_print_counts();

        // TODO.async What meaning should config.workers have now?
        // Initialize the tokio runtime
        let mut runtime = tokio::runtime::Builder::new()
            .core_threads(self.config.workers as usize)
            .build()
            .expect("Cannot build runtime!");

        let threads = self.config.workers as usize;

        let full_addr = format!("{}:{}", self.config.address, self.config.port)
            .to_socket_addrs()
            .expect("A valid socket address")
            .next()
            .unwrap();

        let listener = match TcpListener::bind(&full_addr) {
            Ok(listener) => listener,
            Err(e) => return LaunchError::new(LaunchErrorKind::Bind(e)),
        };

        // Determine the address and port we actually binded to.
        match listener.local_addr() {
            Ok(server_addr) => /* TODO self.config.port = */ server_addr.port(),
            Err(e) => return LaunchError::from(e),
        };

        let proto;
        let incoming;

        #[cfg(feature = "tls")]
        {
            // TODO.async: Can/should we make the clone unnecessary (by reference, or by moving out?)
            if let Some(tls) = self.config.tls.clone() {
                proto = "https://";
                let mut config = tls::rustls::ServerConfig::new(tls::rustls::NoClientAuth::new());
                config.set_single_cert(tls.certs, tls.key).expect("invalid key or certificate");

                // TODO.async: I once observed an unhandled AlertReceived(UnknownCA) but
                // have no idea what happened and cannot reproduce.
                let config = TlsAcceptor::from(Arc::new(config));

                incoming = Box::new(listener.incoming().and_then(move |stream| {
                    config.accept(stream)
                        .map(|stream| Box::new(stream))
                }));
            }
            else {
                proto = "http://";
                incoming = Box::new(listener.incoming().map(|stream| Box::new(stream)));
            }
        }

        #[cfg(not(feature = "tls"))]
        {
            proto = "http://";
            incoming = Box::new(listener.incoming().map(|stream| Box::new(stream)));
        }

        // Freeze managed state for synchronization-free accesses later.
        self.state.freeze();

        // Run the launch fairings.
        self.fairings.handle_launch(&self);

        launch_info!("{}{} {}{}",
                     Paint::masked("🚀 "),
                     Paint::default("Rocket has launched from").bold(),
                     Paint::default(proto).bold().underline(),
                     Paint::default(&full_addr).bold().underline());

        // Restore the log level back to what it originally was.
        logger::pop_max_level();

        let arcs = RocketArcs::from(self);

        // NB: executor must be passed manually here, see hyperium/hyper#1537
        let server = hyper::Server::builder(incoming)
            .executor(runtime.executor())
            .serve(arcs);

        // TODO.async: Use with_graceful_shutdown, and let launch() return a Result<(), Error>
        runtime.block_on(server).expect("TODO.async handle error");

        unreachable!("the call to `handle_threads` should block on success")
    }

    /// Returns an iterator over all of the routes mounted on this instance of
    /// Rocket.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
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
    ///     let rocket = rocket::ignite()
    ///         .mount("/", routes![hello])
    ///         .mount("/hi", routes![hello]);
    ///
    ///     for route in rocket.routes() {
    ///         match route.base() {
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
    pub fn routes<'a>(&'a self) -> impl Iterator<Item = &'a Route> + 'a {
        self.router.routes()
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
    ///
    /// let client = rocket::local::Client::new(rocket).expect("valid rocket");
    /// assert_eq!(client.rocket().state::<MyState>(), Some(&MyState("hello!")));
    /// ```
    #[inline(always)]
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.state.try_get()
    }

    /// Returns the active configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene)]
    /// # #[macro_use] extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Config Printer", |rocket| {
    ///             println!("Rocket launch config: {:?}", rocket.config());
    ///         }))
    ///         .launch();
    /// # }
    /// }
    /// ```
    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }
}

impl From<Rocket> for RocketArcs {
    fn from(mut rocket: Rocket) -> Self {
        RocketArcs {
            config: Arc::new(rocket.config),
            router: Arc::new(rocket.router),
            default_catchers: Arc::new(rocket.default_catchers),
            catchers: Arc::new(rocket.catchers),
            state: Arc::new(rocket.state),
            fairings: Arc::new(rocket.fairings),
        }
    }
}

// TODO: consider try_from here?
impl<'a> From<Response<'a>> for hyper::Response<hyper::Body> {
    fn from(mut response: Response) -> Self {

        let mut builder = hyper::Response::builder();
        builder.status(hyper::StatusCode::from_u16(response.status().code).expect(""));

        for header in response.headers().iter() {
            // FIXME: Using hyper here requires two allocations.
            let name = hyper::HeaderName::from_str(&header.name.into_string()).unwrap();
            let value = hyper::HeaderValue::from_bytes(header.value.as_bytes()).unwrap();
            builder.header(name, value);
        }

        match response.body() {
            None => {
                builder.body(hyper::Body::empty())
            },
            Some(Body::Sized(body, size)) => {
                let mut buffer = Vec::with_capacity(size as usize);
                body.read_to_end(&mut buffer);
                builder.header(header::CONTENT_LENGTH, hyper::HeaderValue::from(size));
                builder.body(hyper::Body::from(buffer))
            },
            Some(Body::Chunked(mut body, chunk_size)) => {
                unimplemented!()
            }
        }.unwrap()
    }
}