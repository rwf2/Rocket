use std::collections::HashMap;
use std::str::from_utf8_unchecked;
use std::cmp::min;
use std::net::SocketAddr;
use std::io::{self, Write};
use std::mem;

use yansi::Paint;
use state::Container;

#[cfg(feature = "tls")] use hyper_sync_rustls::TlsServer;
use {logger, handler};
use ext::ReadExt;
use config::{self, Config, LoggedValue};
use request::{Request, FormItems};
use data::Data;
use response::{Body, Response};
use router::{Router, Route};
use catcher::{self, Catcher};
use outcome::Outcome;
use error::{Error, LaunchError, LaunchErrorKind};
use fairing::{Fairing, Fairings};

use http::{Method, Status, Header};
use http::hyper::{self, header};
use http::uri::Uri;

/// The main `Rocket` type: used to mount routes and catchers and launch the
/// application.
pub struct Rocket {
    pub(crate) config: Config,
    router: Router,
    default_catchers: HashMap<u16, Catcher>,
    catchers: HashMap<u16, Catcher>,
    pub(crate) state: Container,
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
                let dummy = Request::new(self, Method::Get, Uri::new("<unknown>"));
                let r = self.handle_error(Status::InternalServerError, &dummy);
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
    fn issue_response(&self, response: Response, hyp_res: hyper::FreshResponse) {
        match self.write_response(response, hyp_res) {
            Ok(_) => info_!("{}", Paint::green("Response succeeded.")),
            Err(e) => error_!("Failed to write response: {:?}.", e),
        }
    }

    #[inline]
    fn write_response(
        &self,
        mut response: Response,
        mut hyp_res: hyper::FreshResponse,
    ) -> io::Result<()> {
        *hyp_res.status_mut() = hyper::StatusCode::from_u16(response.status().code);

        for header in response.headers().iter() {
            // FIXME: Using hyper here requires two allocations.
            let name = header.name.into_string();
            let value = Vec::from(header.value.as_bytes());
            hyp_res.headers_mut().append_raw(name, value);
        }

        if response.body().is_none() {
            hyp_res.headers_mut().set(header::ContentLength(0));
            return hyp_res.start()?.end();
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
    ///   * Rewriting the remote IP if the 'X-Real-IP' header is set.
    ///
    /// Keep this in-sync with derive_form when preprocessing form fields.
    fn preprocess_request(&self, req: &mut Request, data: &Data) {
        // Rewrite the remote IP address. The request must already have an
        // address associated with it to do this since we need to know the port.
        if let Some(current) = req.remote() {
            let ip = req.headers()
                .get_one("X-Real-IP")
                .and_then(|ip| {
                    ip.parse()
                        .map_err(|_| warn_!("'X-Real-IP' header is malformed: {}", ip))
                        .ok()
                });

            if let Some(ip) = ip {
                req.set_remote(SocketAddr::new(ip, current.port()));
            }
        }

        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        let data_len = data.peek().len();
        let (min_len, max_len) = ("_method=get".len(), "_method=delete".len());
        let is_form = req.content_type().map_or(false, |ct| ct.is_form());
        if is_form && req.method() == Method::Post && data_len >= min_len {
            // We're only using this for comparison and throwing it away
            // afterwards, so it doesn't matter if we have invalid UTF8.
            let form =
                unsafe { from_utf8_unchecked(&data.peek()[..min(data_len, max_len)]) };

            if let Some((key, value)) = FormItems::from(form).next() {
                if key == "_method" {
                    if let Ok(method) = value.parse() {
                        req.set_method(method);
                    }
                }
            }
        }
    }

    #[inline]
    pub(crate) fn dispatch<'s, 'r>(
        &'s self,
        request: &'r mut Request<'s>,
        data: Data,
    ) -> Response<'r> {
        info!("{}:", request);

        // Do a bit of preprocessing before routing; run the attached fairings.
        self.preprocess_request(request, &data);
        self.fairings.handle_request(request, &data);

        // Route the request to get a response.
        let mut response = match self.route(request, data) {
            Outcome::Success(mut response) => {
                // A user's route responded! Set the cookies.
                for cookie in request.cookies().delta() {
                    response.adjoin_header(cookie);
                }

                response
            }
            Outcome::Forward(data) => {
                // Rust thinks `request` is still borrowed here, but it's
                // obviously not (data has nothing to do with it), so we
                // convince it to give us another mutable reference.
                // TODO: Use something that is well defined, like UnsafeCell.
                // But that causes variance issues...so wait for NLL.
                let request: &'r mut Request<'s> =
                    unsafe { (&mut *(request as *const _ as *mut _)) };

                // There was no matching route.
                if request.method() == Method::Head {
                    info_!("Autohandling {} request.", Paint::white("HEAD"));
                    request.set_method(Method::Get);
                    let mut response = self.dispatch(request, data);
                    response.strip_body();
                    response
                } else {
                    self.handle_error(Status::NotFound, request)
                }
            }
            Outcome::Failure(status) => self.handle_error(status, request),
        };

        // Add the 'rocket' server header to the response and run fairings.
        // TODO: If removing Hyper, write out `Date` header too.
        response.set_header(Header::new("Server", "Rocket"));
        self.fairings.handle_response(request, &mut response);

        response
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
    pub(crate) fn route<'s, 'r>(
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
            let outcome = (route.handler)(request, data);

            // Check if the request processing completed or if the request needs
            // to be forwarded. If it does, continue the loop to try again.
            info_!("{} {}", Paint::white("Outcome:"), outcome);
            match outcome {
                o@Outcome::Success(_) | o@Outcome::Failure(_) => return o,
                Outcome::Forward(unused_data) => data = unused_data,
            };
        }

        error_!("No matching routes for {}.", request);
        Outcome::Forward(data)
    }

    // Finds the error catcher for the status `status` and executes it fo the
    // given request `req`. If a user has registere a catcher for `status`, the
    // catcher is called. If the catcher fails to return a good response, the
    // 500 catcher is executed. if there is no registered catcher for `status`,
    // the default catcher is used.
    fn handle_error<'r>(&self, status: Status, req: &'r Request) -> Response<'r> {
        warn_!("Responding with {} catcher.", Paint::red(&status));

        // Try to get the active catcher but fallback to user's 500 catcher.
        let catcher = self.catchers.get(&status.code).unwrap_or_else(|| {
            error_!("No catcher found for {}. Using 500 catcher.", status);
            self.catchers.get(&500).expect("500 catcher.")
        });

        // Dispatch to the user's catcher. If it fails, use the default 500.
        let error = Error::NoRoute;
        catcher.handle(error, req).unwrap_or_else(|err_status| {
            error_!("Catcher failed with status: {}!", err_status);
            warn_!("Using default 500 error catcher.");
            let default = self.default_catchers.get(&500).expect("Default 500");
            default.handle(error, req).expect("Default 500 response.")
        })
    }

    /// Create a new `Rocket` application using the configuration information in
    /// `Rocket.toml`. If the file does not exist or if there is an I/O error
    /// reading the file, the defaults are used. See the
    /// [config](/rocket/config/index.html) documentation for more information
    /// on defaults.
    ///
    /// This method is typically called through the `rocket::ignite` alias.
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
        Rocket::configured(config::init(), true)
    }

    /// Creates a new `Rocket` application using the supplied custom
    /// configuration information. The `Rocket.toml` file, if present, is
    /// ignored. Any environment variables setting config parameters are
    /// ignored. If `log` is `true`, logging is enabled.
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
    /// let app = rocket::custom(config, false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn custom(config: Config, log: bool) -> Rocket {
        Rocket::configured(config, log)
    }

    #[inline]
    fn configured(config: Config, log: bool) -> Rocket {
        if log {
            logger::try_init(config.log_level, false);
        }

        info!("{}Configured for {}.", Paint::masked("🔧  "), config.environment);
        info_!("address: {}", Paint::white(&config.address));
        info_!("port: {}", Paint::white(&config.port));
        info_!("log: {}", Paint::white(config.log_level));
        info_!("workers: {}", Paint::white(config.workers));
        info_!("secret key: {}", Paint::white(&config.secret_key));
        info_!("limits: {}", Paint::white(&config.limits));

        let tls_configured = config.tls.is_some();
        if tls_configured && cfg!(feature = "tls") {
            info_!("tls: {}", Paint::white("enabled"));
        } else if tls_configured {
            error_!("tls: {}", Paint::white("disabled"));
            error_!("tls is configured, but the tls feature is disabled");
        } else {
            info_!("tls: {}", Paint::white("disabled"));
        }

        if config.secret_key.is_generated() && config.environment.is_prod() {
            warn!("environment is 'production', but no `secret_key` is configured");
        }

        for (name, value) in config.extras() {
            info_!("{} {}: {}",
                   Paint::yellow("[extra]"),
                   Paint::blue(name),
                   Paint::white(LoggedValue(value)));
        }

        Rocket {
            config: config,
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
    /// The `base` mount point must be a static path. That is, the mount point
    /// must _not_ contain dynamic path parameters: `<param>`.
    ///
    /// # Examples
    ///
    /// Use the `routes!` macro to mount routes created using the code
    /// generation facilities. Requests to the `/hello/world` URI will be
    /// dispatched to the `hi` route.
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
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
    /// fn hi(req: &Request, _: Data) -> Outcome<'static> {
    ///     Outcome::from(req, "Hello!")
    /// }
    ///
    /// # if false { // We don't actually want to launch the server in an example.
    /// rocket::ignite().mount("/hello", vec![Route::new(Get, "/world", hi)])
    /// #     .launch();
    /// # }
    /// ```
    #[inline]
    pub fn mount(mut self, base: &str, routes: Vec<Route>) -> Self {
        info!("{}{} '{}':",
              Paint::masked("🛰  "),
              Paint::purple("Mounting"),
              Paint::blue(base));

        if base.contains('<') || !base.starts_with('/') {
            error_!("Bad mount point: '{}'.", base);
            error_!("Mount points must be static, absolute URIs: `/example`");
            panic!("Bad mount point.")
        }

        for mut route in routes {
            let uri = Uri::new(format!("{}/{}", base, route.uri));

            route.set_base(base);
            route.set_uri(uri.to_string());

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
    /// #![feature(plugin, decl_macro)]
    /// #![plugin(rocket_codegen)]
    ///
    /// extern crate rocket;
    ///
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
    ///     rocket::ignite().catch(catchers![internal_error, not_found])
    /// #       .launch();
    /// # }
    /// }
    /// ```
    #[inline]
    pub fn catch(mut self, catchers: Vec<Catcher>) -> Self {
        info!("👾  {}:", Paint::purple("Catchers"));
        for c in catchers {
            if self.catchers.get(&c.code).map_or(false, |e| !e.is_default()) {
                let msg = "(warning: duplicate catcher!)";
                info_!("{} {}", c, Paint::yellow(msg));
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
    /// [State](/rocket/struct.State.html) request guard. In particular, if a
    /// value of type `T` is managed by Rocket, adding `State<T>` to the list of
    /// arguments in a request handler instructs Rocket to retrieve the managed
    /// value.
    ///
    /// # Panics
    ///
    /// Panics if state of type `T` is already being managed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
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

    /// Attaches a fairing to this instance of Rocket.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch(|_| println!("Rocket is launching!")))
    ///         .launch();
    /// # }
    /// }
    /// ```
    #[inline]
    pub fn attach<F: Fairing>(mut self, fairing: F) -> Self {
        // Attach the fairings, which requires us to move `self`.
        let mut fairings = mem::replace(&mut self.fairings, Fairings::new());
        self = fairings.attach(Box::new(fairing), self);

        // Make sure we keep the fairings around!
        self.fairings = fairings;
        self
    }

    pub(crate) fn prelaunch_check(&self) -> Option<LaunchError> {
        let collisions = self.router.collisions();
        if !collisions.is_empty() {
            let owned = collisions.iter().map(|&(a, b)| (a.clone(), b.clone()));
            Some(LaunchError::from(LaunchErrorKind::Collision(owned.collect())))
        } else if self.fairings.had_failure() {
            Some(LaunchError::from(LaunchErrorKind::FailedFairing))
        } else {
            None
        }
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
    /// [`LaunchError`]: /rocket/error/struct.LaunchError.html
    ///
    /// # Example
    ///
    /// ```rust
    /// # if false {
    /// rocket::ignite().launch();
    /// # }
    /// ```
    pub fn launch(mut self) -> LaunchError {
        if let Some(error) = self.prelaunch_check() {
            return error;
        }

        self.fairings.pretty_print_counts();

        let full_addr = format!("{}:{}", self.config.address, self.config.port);
        serve!(self, &full_addr, |server, proto| {
            let mut server = match server {
                Ok(server) => server,
                Err(e) => return LaunchError::from(e),
            };

            // Determine the address and port we actually binded to.
            match server.local_addr() {
                Ok(server_addr) => self.config.port = server_addr.port(),
                Err(e) => return LaunchError::from(e),
            }

            // Run the launch fairings.
            self.fairings.handle_launch(&self);

            let full_addr = format!("{}:{}", self.config.address, self.config.port);
            launch_info!("{}{} {}{}",
                         Paint::masked("🚀  "),
                         Paint::white("Rocket has launched from"),
                         Paint::white(proto).bold(),
                         Paint::white(&full_addr).bold());

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
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
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

    /// Returns the active configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// fn main() {
    /// # if false { // We don't actually want to launch the server in an example.
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch(|rocket| {
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
