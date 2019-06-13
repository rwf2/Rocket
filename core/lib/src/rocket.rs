use std::collections::HashMap;
use std::str::from_utf8;
use std::cmp::min;
use std::io::{self, Write};
use std::time::Duration;
use std::mem;

use yansi::Paint;
use state::Container;

#[cfg(feature = "tls")] use crate::http::tls::TlsServer;

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

#[doc(hidden)]
impl hyper::Handler for Rocket {
    // This function tries to hide all of the Hyper-ness from Rocket. It
    // essentially converts Hyper types into Rocket types, then calls the
    // `dispatch` function, which knows nothing about Hyper. Because responding
    // depends on the `HyperResponse` type, this function does the actual
    // response processing.
    fn handle<'h, 'k>(
        &self,
        hyp_req: hyper::Request<'h, 'k>,
        res: hyper::FreshResponse<'h>,
    ) {
        // Get all of the information from Hyper.
        let (h_addr, h_method, h_headers, h_uri, _, h_body) = hyp_req.deconstruct();

        // Convert the Hyper request into a Rocket request.
        let req_res = Request::from_hyp(self, h_method, h_headers, h_uri, h_addr);
        let mut req = match req_res {
            Ok(req) => req,
            Err(e) => {
                error!("Bad incoming request: {}", e);
                // TODO: We don't have a request to pass in, so we just
                // fabricate one. This is weird. We should let the user know
                // that we failed to parse a request (by invoking some special
                // handler) instead of doing this.
                let dummy = Request::new(self, Method::Get, Origin::dummy());
                let r = self.handle_error(Status::BadRequest, &dummy);
                return self.issue_response(r, res);
            }
        };

        // Retrieve the data from the hyper body.
        let data = match Data::from_hyp(h_body) {
            Ok(data) => data,
            Err(reason) => {
                error_!("Bad data in request: {}", reason);
                let r = self.handle_error(Status::InternalServerError, &req);
                return self.issue_response(r, res);
            }
        };

        // Dispatch the request to get a response, then write that response out.
        let response = self.dispatch(&mut req, data);
        self.issue_response(response, res)
    }
}

// This macro is a terrible hack to get around Hyper's Server<L> type. What we
// want is to use almost exactly the same launch code when we're serving over
// HTTPS as over HTTP. But Hyper forces two different types, so we can't use the
// same code, at least not trivially. These macros get around that by passing in
// the same code as a continuation in `$continue`. This wouldn't work as a
// regular function taking in a closure because the types of the inputs to the
// closure would be different depending on whether TLS was enabled or not.
#[cfg(not(feature = "tls"))]
macro_rules! serve {
    ($rocket:expr, $addr:expr, |$server:ident, $proto:ident| $continue:expr) => ({
        let ($proto, $server) = ("http://", hyper::Server::http($addr));
        $continue
    })
}

#[cfg(feature = "tls")]
macro_rules! serve {
    ($rocket:expr, $addr:expr, |$server:ident, $proto:ident| $continue:expr) => ({
        if let Some(tls) = $rocket.config.tls.clone() {
            let tls = TlsServer::new(tls.certs, tls.key);
            let ($proto, $server) = ("https://", hyper::Server::https($addr, tls));
            $continue
        } else {
            let ($proto, $server) = ("http://", hyper::Server::http($addr));
            $continue
        }
    })
}

impl Rocket {
    #[inline]
    fn issue_response(&self, response: Response<'_>, hyp_res: hyper::FreshResponse<'_>) {
        match self.write_response(response, hyp_res) {
            Ok(_) => info_!("{}", Paint::green("Response succeeded.")),
            Err(e) => error_!("Failed to write response: {:?}.", e),
        }
    }

    #[inline]
    fn write_response(
        &self,
        mut response: Response<'_>,
        mut hyp_res: hyper::FreshResponse<'_>,
    ) -> io::Result<()> {
        *hyp_res.status_mut() = hyper::StatusCode::from_u16(response.status().code);

        for header in response.headers().iter() {
            // FIXME: Using hyper here requires two allocations.
            let name = header.name.into_string();
            let value = Vec::from(header.value.as_bytes());
            hyp_res.headers_mut().append_raw(name, value);
        }

        match response.body() {
            None => {
                hyp_res.headers_mut().set(header::ContentLength(0));
                hyp_res.start()?.end()
            }
            Some(Body::Sized(body, size)) => {
                hyp_res.headers_mut().set(header::ContentLength(size));
                let mut stream = hyp_res.start()?;
                io::copy(body, &mut stream)?;
                stream.end()
            }
            Some(Body::Chunked(mut body, chunk_size)) => {
                // This _might_ happen on a 32-bit machine!
                if chunk_size > (usize::max_value() as u64) {
                    let msg = "chunk size exceeds limits of usize type";
                    return Err(io::Error::new(io::ErrorKind::Other, msg));
                }

                // The buffer stores the current chunk being written out.
                let mut buffer = vec![0; chunk_size as usize];
                let mut stream = hyp_res.start()?;
                loop {
                    match body.read_max(&mut buffer)? {
                        0 => break,
                        n => stream.write_all(&buffer[..n])?,
                    }
                }

                stream.end()
            }
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
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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

        for mut route in routes.into() {
            let path = route.uri.clone();
            if let Err(e) = route.set_uri(base_uri.clone(), path) {
                error_!("{}", e);
                panic!("Invalid route URI.");
            }

            info_!("{}", route);
            self.router.add(route);
        }

        self
    }

    /// Registers all of the catchers in the supplied vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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
        for c in catchers {
            if self.catchers.get(&c.code).map_or(false, |e| !e.is_default) {
                info_!("{} {}", c, Paint::yellow("(warning: duplicate catcher!)"));
            } else {
                info_!("{}", c);
            }

            self.catchers.insert(c.code, c);
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
    /// ```rust
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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

        let full_addr = format!("{}:{}", self.config.address, self.config.port);
        serve!(self, &full_addr, |server, proto| {
            let mut server = match server {
                Ok(server) => server,
                Err(e) => return LaunchError::new(LaunchErrorKind::Bind(e)),
            };

            // Determine the address and port we actually binded to.
            match server.local_addr() {
                Ok(server_addr) => self.config.port = server_addr.port(),
                Err(e) => return LaunchError::from(e),
            }

            // Set the keep-alive.
            let timeout = self.config.keep_alive.map(|s| Duration::from_secs(s as u64));
            server.keep_alive(timeout);

            // Freeze managed state for synchronization-free accesses later.
            self.state.freeze();

            // Run the launch fairings.
            self.fairings.handle_launch(&self);

            let full_addr = format!("{}:{}", self.config.address, self.config.port);
            launch_info!("{}{} {}{}",
                         Paint::masked("🚀 "),
                         Paint::default("Rocket has launched from").bold(),
                         Paint::default(proto).bold().underline(),
                         Paint::default(&full_addr).bold().underline());

            // Restore the log level back to what it originally was.
            logger::pop_max_level();

            let threads = self.config.workers as usize;
            if let Err(e) = server.handle_threads(self, threads) {
                return LaunchError::from(e);
            }

            unreachable!("the call to `handle_threads` should block on success")
        })
    }

    /// Returns an iterator over all of the routes mounted on this instance of
    /// Rocket.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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
    /// # #![feature(proc_macro_hygiene, decl_macro)]
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
