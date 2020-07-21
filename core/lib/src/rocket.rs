use std::{io, mem};
use std::cmp::min;
use std::sync::Arc;
use std::collections::HashMap;

#[allow(unused_imports)]
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::future::{Future, BoxFuture};
use tokio::sync::{mpsc, oneshot};
use ref_cast::RefCast;

use yansi::Paint;
use state::Container;

use crate::{logger, handler};
use crate::config::{Config, FullConfig, ConfigError, LoggedValue};
use crate::request::{Request, FormItems};
use crate::data::Data;
use crate::response::{Body, Response};
use crate::router::{Router, Route};
use crate::catcher::{self, Catcher};
use crate::outcome::Outcome;
use crate::error::{LaunchError, LaunchErrorKind};
use crate::fairing::{Fairing, Fairings};
use crate::logger::PaintExt;
use crate::ext::AsyncReadExt;
use crate::shutdown::{ShutdownHandle, ShutdownHandleManaged};

use crate::http::{Method, Status, Header};
use crate::http::private::{Listener, Connection, Incoming};
use crate::http::hyper::{self, header};
use crate::http::uri::Origin;

/// The main `Rocket` type: used to mount routes and catchers and launch the
/// application.
pub struct Rocket {
    manifest: Vec<PreLaunchOp>,
    pub(crate) config: Config,
    router: Router,
    default_catchers: HashMap<u16, Catcher>,
    catchers: HashMap<u16, Catcher>,
    pub(crate) managed_state: Container,
    fairings: Fairings,
    shutdown_handle: ShutdownHandle,
    shutdown_receiver: Option<mpsc::Receiver<()>>,
}

/// An operation that occurs prior to launching a Rocket instance.
enum PreLaunchOp {
    Mount(Origin<'static>, Vec<Route>),
    Register(Vec<Catcher>),
    Manage(&'static str, Box<dyn FnOnce(&mut Container) + Send + Sync + 'static>),
    Attach(Box<dyn Fairing>),
}

/// A frozen view into the contents of an instance of `Rocket`.
///
/// Obtained via [`Rocket::inspect()`].
#[derive(RefCast)]
#[repr(transparent)]
pub struct Cargo(Rocket);

// A token returned to force the execution of one method before another.
pub(crate) struct Token;

impl Rocket {
    #[inline]
    fn _mount(&mut self, base: Origin<'static>, routes: Vec<Route>) {
        info!("{}{} {}{}",
              Paint::emoji("🛰  "),
              Paint::magenta("Mounting"),
              Paint::blue(&base),
              Paint::magenta(":"));

        for mut route in routes {
            let path = route.uri.clone();
            if let Err(e) = route.set_uri(base.clone(), path) {
                error_!("{}", e);
                panic!("Invalid route URI.");
            }

            info_!("{}", route);
            self.router.add(route);
        }
    }

    #[inline]
    fn _register(&mut self, catchers: Vec<Catcher>) {
        info!("{}{}", Paint::emoji("👾 "), Paint::magenta("Catchers:"));

        for c in catchers {
            if self.catchers.get(&c.code).map_or(false, |e| !e.is_default) {
                info_!("{} {}", c, Paint::yellow("(warning: duplicate catcher!)"));
            } else {
                info_!("{}", c);
            }

            self.catchers.insert(c.code, c);
        }
    }

    #[inline]
    async fn _attach(mut self, fairing: Box<dyn Fairing>) -> Self {
        // Attach (and run attach-) fairings, which requires us to move `self`.
        let mut fairings = mem::replace(&mut self.fairings, Fairings::new());
        self = fairings.attach(fairing, self).await;

        // Note that `self.fairings` may now be non-empty! Move them to the end.
        fairings.append(self.fairings);
        self.fairings = fairings;
        self
    }

    // Create a "dummy" instance of `Rocket` to use while mem-swapping `self`.
    fn dummy() -> Rocket {
        Rocket {
            manifest: vec![],
            config: Config::development(),
            router: Router::new(),
            default_catchers: HashMap::new(),
            catchers: HashMap::new(),
            managed_state: Container::new(),
            fairings: Fairings::new(),
            shutdown_handle: ShutdownHandle(mpsc::channel(1).0),
            shutdown_receiver: None,
        }
    }

    // Instead of requiring the user to individually `await` each call to
    // `attach()`, some operations are queued in `self.pending`. Functions that
    // want to provide read access to any data from the Cargo, such as
    // `inspect()`, need to apply those pending operations first.
    //
    // This function returns a future that executes those pending operations,
    // requiring only a single `await` at the call site. After completion,
    // `self.pending` will be empty and `self.manifest` will reflect all pending
    // changes.
    async fn actualize_manifest(&mut self) {
        // Note: attach fairings may add more ops to the `manifest`! We
        // process them as a stack to maintain proper ordering.
        let mut manifest = mem::replace(&mut self.manifest, vec![]);
        while !manifest.is_empty() {
            trace_!("[MANIEST PROGRESS]: {:?}", manifest);
            match manifest.remove(0) {
                PreLaunchOp::Manage(_, callback) => callback(&mut self.managed_state),
                PreLaunchOp::Mount(base, routes) => self._mount(base, routes),
                PreLaunchOp::Register(catchers) => self._register(catchers),
                PreLaunchOp::Attach(fairing) => {
                    let rocket = mem::replace(self, Rocket::dummy());
                    *self = rocket._attach(fairing).await;
                    self.manifest.append(&mut manifest);
                    manifest = mem::replace(&mut self.manifest, vec![]);
                }
            }
        }
    }

    pub(crate) async fn into_cargo(mut self) -> Cargo {
        self.actualize_manifest().await;
        Cargo(self)
    }

    fn cargo(&self) -> &Cargo {
        if !self.manifest.is_empty() {
            panic!("internal error: immutable launch state with manifest");
        }

        Cargo::ref_cast(self)
    }
}

// This function tries to hide all of the Hyper-ness from Rocket. It essentially
// converts Hyper types into Rocket types, then calls the `dispatch` function,
// which knows nothing about Hyper. Because responding depends on the
// `HyperResponse` type, this function does the actual response processing.
async fn hyper_service_fn(
    rocket: Arc<Rocket>,
    h_addr: std::net::SocketAddr,
    hyp_req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, io::Error> {
    // This future must return a hyper::Response, but that's not easy
    // because the response body might borrow from the request. Instead,
    // we do the body writing in another future that will send us
    // the response metadata (and a body channel) beforehand.
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        // Get all of the information from Hyper.
        let (h_parts, h_body) = hyp_req.into_parts();

        // Convert the Hyper request into a Rocket request.
        let req_res = Request::from_hyp(&rocket, h_parts.method, h_parts.headers, &h_parts.uri, h_addr);
        let mut req = match req_res {
            Ok(req) => req,
            Err(e) => {
                error!("Bad incoming request: {}", e);
                // TODO: We don't have a request to pass in, so we just
                // fabricate one. This is weird. We should let the user know
                // that we failed to parse a request (by invoking some special
                // handler) instead of doing this.
                let dummy = Request::new(&rocket, Method::Get, Origin::dummy());
                let r = rocket.handle_error(Status::BadRequest, &dummy).await;
                return rocket.issue_response(r, tx).await;
            }
        };

        // Retrieve the data from the hyper body.
        let data = Data::from_hyp(h_body).await;

        // Dispatch the request to get a response, then write that response out.
        let token = rocket.preprocess_request(&mut req, &data).await;
        let r = rocket.dispatch(token, &mut req, data).await;
        rocket.issue_response(r, tx).await;
    });

    rx.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

impl Rocket {
    #[inline]
    async fn issue_response(
        &self,
        response: Response<'_>,
        tx: oneshot::Sender<hyper::Response<hyper::Body>>,
    ) {
        let result = self.write_response(response, tx);
        match result.await {
            Ok(()) => {
                info_!("{}", Paint::green("Response succeeded."));
            }
            Err(e) => {
                error_!("Failed to write response: {:?}.", e);
            }
        }
    }

    #[inline]
    async fn write_response(
        &self,
        mut response: Response<'_>,
        tx: oneshot::Sender<hyper::Response<hyper::Body>>,
    ) -> io::Result<()> {
        let mut hyp_res = hyper::Response::builder()
            .status(response.status().code);

        for header in response.headers().iter() {
            let name = header.name.as_str();
            let value = header.value.as_bytes();
            hyp_res = hyp_res.header(name, value);
        }

        let send_response = move |hyp_res: hyper::ResponseBuilder, body| -> io::Result<()> {
            let response = hyp_res.body(body)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            tx.send(response).map_err(|_| {
                let msg =  "Client disconnected before the response was started";
                io::Error::new(io::ErrorKind::BrokenPipe, msg)
            })
        };

        match response.body_mut() {
            None => {
                hyp_res = hyp_res.header(header::CONTENT_LENGTH, "0");
                send_response(hyp_res, hyper::Body::empty())?;
            }
            Some(body) => {
                if let Some(s) = body.size().await {
                    hyp_res = hyp_res.header(header::CONTENT_LENGTH, s.to_string());
                }

                let chunk_size = match *body {
                    Body::Chunked(_, chunk_size) => chunk_size as usize,
                    Body::Sized(_, _) => crate::response::DEFAULT_CHUNK_SIZE,
                };

                let (mut sender, hyp_body) = hyper::Body::channel();
                send_response(hyp_res, hyp_body)?;

                let mut stream = body.as_reader().into_bytes_stream(chunk_size);
                while let Some(next) = stream.next().await {
                    sender.send_data(next?).await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }
            }
        };

        Ok(())
    }

    /// Preprocess the request for Rocket things. Currently, this means:
    ///
    ///   * Rewriting the method in the request if _method form field exists.
    ///   * Run the request fairings.
    ///
    /// Keep this in-sync with derive_form when preprocessing form fields.
    pub(crate) async fn preprocess_request(
        &self,
        req: &mut Request<'_>,
        data: &Data
    ) -> Token {
        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        let data_len = data.peek().len();
        let (min_len, max_len) = ("_method=get".len(), "_method=delete".len());
        let is_form = req.content_type().map_or(false, |ct| ct.is_form());

        if is_form && req.method() == Method::Post && data_len >= min_len {
            if let Ok(form) = std::str::from_utf8(&data.peek()[..min(data_len, max_len)]) {
                let method: Option<Result<Method, _>> = FormItems::from(form)
                    .filter(|item| item.key.as_str() == "_method")
                    .map(|item| item.value.parse())
                    .next();

                if let Some(Ok(method)) = method {
                    req._set_method(method);
                }
            }
        }

        // Run request fairings.
        self.fairings.handle_request(req, data).await;

        Token
    }

    /// Route the request and process the outcome to eventually get a response.
    fn route_and_process<'s, 'r: 's>(
        &'s self,
        request: &'r Request<'s>,
        data: Data
    ) -> impl Future<Output = Response<'r>> + Send + 's {
        async move {
            let mut response = match self.route(request, data).await {
                Outcome::Success(response) => response,
                Outcome::Forward(data) => {
                    // There was no matching route. Autohandle `HEAD` requests.
                    if request.method() == Method::Head {
                        info_!("Autohandling {} request.", Paint::default("HEAD").bold());

                        // Dispatch the request again with Method `GET`.
                        request._set_method(Method::Get);

                        // Return early so we don't set cookies twice.
                        let try_next: BoxFuture<'_, _> = Box::pin(self.route_and_process(request, data));
                        return try_next.await;
                    } else {
                        // No match was found and it can't be autohandled. 404.
                        self.handle_error(Status::NotFound, request).await
                    }
                }
                Outcome::Failure(status) => self.handle_error(status, request).await,
            };

            // Set the cookies. Note that error responses will only include
            // cookies set by the error handler. See `handle_error` for more.
            for cookie in request.cookies().delta() {
                response.adjoin_header(cookie);
            }

            response
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
    pub(crate) fn route<'s, 'r: 's>(
        &'s self,
        request: &'r Request<'s>,
        mut data: Data,
    ) -> impl Future<Output = handler::Outcome<'r>> + 's {
        async move {
            // Go through the list of matching routes until we fail or succeed.
            let matches = self.router.route(request);
            for route in matches {
                // Retrieve and set the requests parameters.
                info_!("Matched: {}", route);
                request.set_route(route);

                // Dispatch the request to the handler.
                let outcome = route.handler.handle(request, data).await;

                // Check if the request processing completed (Some) or if the
                // request needs to be forwarded. If it does, continue the loop
                // (None) to try again.
                info_!("{} {}", Paint::default("Outcome:").bold(), outcome);
                match outcome {
                    o@Outcome::Success(_) | o@Outcome::Failure(_) => return o,
                    Outcome::Forward(unused_data) => data = unused_data,
                }
            }

            error_!("No matching routes for {}.", request);
            Outcome::Forward(data)
        }
    }

    #[inline]
    pub(crate) async fn dispatch<'s, 'r: 's>(
        &'s self,
        _token: Token,
        request: &'r Request<'s>,
        data: Data
    ) -> Response<'r> {
        info!("{}:", request);

        // Remember if the request is `HEAD` for later body stripping.
        let was_head_request = request.method() == Method::Head;

        // Route the request and run the user's handlers.
        let mut response = self.route_and_process(request, data).await;

        // Add a default 'Server' header if it isn't already there.
        // TODO: If removing Hyper, write out `Date` header too.
        if !response.headers().contains("Server") {
            response.set_header(Header::new("Server", "Rocket"));
        }

        // Run the response fairings.
        self.fairings.handle_response(request, &mut response).await;

        // Strip the body if this is a `HEAD` request.
        if was_head_request {
            response.strip_body();
        }

        response
    }

    // Finds the error catcher for the status `status` and executes it for the
    // given request `req`. If a user has registered a catcher for `status`, the
    // catcher is called. If the catcher fails to return a good response, the
    // 500 catcher is executed. If there is no registered catcher for `status`,
    // the default catcher is used.
    pub(crate) fn handle_error<'s, 'r: 's>(
        &'s self,
        status: Status,
        req: &'r Request<'s>
    ) -> impl Future<Output = Response<'r>> + 's {
        async move {
            warn_!("Responding with {} catcher.", Paint::red(&status));

            // For now, we reset the delta state to prevent any modifications
            // from earlier, unsuccessful paths from being reflected in error
            // response. We may wish to relax this in the future.
            req.cookies().reset_delta();

            // Try to get the active catcher but fallback to user's 500 catcher.
            let catcher = self.catchers.get(&status.code).unwrap_or_else(|| {
                error_!("No catcher found for {}. Using 500 catcher.", status);
                self.catchers.get(&500).expect("500 catcher.")
            });

            // Dispatch to the user's catcher. If it fails, use the default 500.
            match catcher.handle(req).await {
                Ok(r) => return r,
                Err(err_status) => {
                    error_!("Catcher failed with status: {}!", err_status);
                    warn_!("Using default 500 error catcher.");
                    let default = self.default_catchers.get(&500).expect("Default 500");
                    default.handle(req).await.expect("Default 500 response.")
                }
            }
        }
    }

    // TODO.async: Solidify the Listener APIs and make this function public
    async fn listen_on<L>(mut self, listener: L) -> Result<(), crate::error::Error>
        where L: Listener + Send + Unpin + 'static,
              <L as Listener>::Connection: Send + Unpin + 'static,
    {
        // Determine the address and port we actually binded to.
        self.config.port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
        let proto = self.config.tls.as_ref().map_or("http://", |_| "https://");
        let full_addr = format!("{}:{}", self.config.address, self.config.port);

        // Freeze managed state for synchronization-free accesses later.
        self.managed_state.freeze();

        // Run the launch fairings.
        self.fairings.pretty_print_counts();
        self.fairings.handle_launch(self.cargo());

        launch_info!("{}{} {}{}",
                     Paint::emoji("🚀 "),
                     Paint::default("Rocket has launched from").bold(),
                     Paint::default(proto).bold().underline(),
                     Paint::default(&full_addr).bold().underline());

        // Restore the log level back to what it originally was.
        logger::pop_max_level();

        // Set the keep-alive.
        // TODO.async: implement keep-alive in Listener
        // let timeout = self.config.keep_alive.map(|s| Duration::from_secs(s as u64));
        // listener.set_keepalive(timeout);

        // We need to get this before moving `self` into an `Arc`.
        let mut shutdown_receiver = self.shutdown_receiver
            .take().expect("shutdown receiver has already been used");

        let rocket = Arc::new(self);
        let service = hyper::make_service_fn(move |connection: &<L as Listener>::Connection| {
            let rocket = rocket.clone();
            let remote_addr = connection.remote_addr().unwrap_or_else(|| ([0, 0, 0, 0], 0).into());
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service_fn(move |req| {
                    hyper_service_fn(rocket.clone(), remote_addr, req)
                }))
            }
        });

        #[derive(Clone)]
        struct TokioExecutor;

        impl<Fut> hyper::Executor<Fut> for TokioExecutor
            where Fut: Future + Send + 'static, Fut::Output: Send
        {
            fn execute(&self, fut: Fut) {
                tokio::spawn(fut);
            }
        }

        hyper::Server::builder(Incoming::from_listener(listener))
            .executor(TokioExecutor)
            .serve(service)
            .with_graceful_shutdown(async move { shutdown_receiver.recv().await; })
            .await
            .map_err(|e| crate::error::Error::Run(Box::new(e)))
    }
}

impl Rocket {
    /// Create a new `Rocket` application using the configuration information in
    /// `Rocket.toml`. If the file does not exist or if there is an I/O error
    /// reading the file, the defaults, overridden by any environment-based
    /// paramparameters, are used. See the [`config`](crate::config)
    /// documentation for more information on defaults.
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
    pub fn ignite() -> Rocket {
        Config::read()
            .or_else(|e| match e {
                ConfigError::IoError => {
                    warn!("Failed to read 'Rocket.toml'. Using defaults.");
                    Ok(FullConfig::env_default()?.take_active())
                }
                ConfigError::NotFound => Ok(FullConfig::env_default()?.take_active()),
                _ => Err(e)
            })
            .map(Rocket::configured)
            .unwrap_or_else(|e: ConfigError| {
                logger::init(logger::LoggingLevel::Debug);
                e.pretty_print();
                std::process::exit(1)
            })
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
        crate::trace::try_init(config.log_level);
        // if logger::try_init(config.log_level, false) {
        //     // Temporary weaken log level for launch info.
        //     logger::push_max_level(logger::LoggingLevel::Normal);
        // }
        let span = tracing::info_span!("Configured", "for {} {}", config.environment, Paint::emoji("🔧 "));
        let _e = span.enter();

        tracing::info!(address = %&config.address);
        tracing::info!(port = %&config.port);
        tracing::info!(log = %&config.log_level);
        tracing::info!(workers = %&config.workers);
        tracing::info!(secret_key = %&config.secret_key);
        tracing::info!(limits = %&config.limits);

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

        let managed_state = Container::new();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);
        let shutdown_handle = ShutdownHandle(shutdown_sender);
        managed_state.set(ShutdownHandleManaged(shutdown_handle.clone()));

        Rocket {
            config, managed_state, shutdown_handle,
            manifest: vec![],
            router: Router::new(),
            default_catchers: catcher::defaults::get(),
            catchers: catcher::defaults::get(),
            fairings: Fairings::new(),
            shutdown_receiver: Some(shutdown_receiver),
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
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// #
    /// #[get("/world")]
    /// fn hi() -> &'static str {
    ///     "Hello!"
    /// }
    ///
    /// #[rocket::launch]
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
    #[inline]
    pub fn mount<R: Into<Vec<Route>>>(mut self, base: &str, routes: R) -> Self {
        let base_uri = Origin::parse_owned(base.to_string())
            .unwrap_or_else(|e| {
                error_!("Invalid origin URI '{}' used as mount point.", base);
                panic!("Error: {}", e);
            });

        if base_uri.query().is_some() {
            error_!("Mount point '{}' contains query string.", base);
            panic!("Invalid mount point.");
        }

        self.manifest.push(PreLaunchOp::Mount(base_uri, routes.into()));
        self
    }

    /// Registers all of the catchers in the supplied vector.
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
    /// #[rocket::launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite().register(catchers![internal_error, not_found])
    /// }
    /// ```
    #[inline]
    pub fn register(mut self, catchers: Vec<Catcher>) -> Self {
        self.manifest.push(PreLaunchOp::Register(catchers));
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
    /// #[rocket::launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .mount("/", routes![index])
    ///         .manage(MyValue(10))
    /// }
    /// ```
    #[inline]
    pub fn manage<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        let type_name = std::any::type_name::<T>();
        self.manifest.push(PreLaunchOp::Manage(type_name, Box::new(move |managed| {
            if !managed.set::<T>(state) {
                error!("State for type '{}' is already being managed!", type_name);
                panic!("Aborting due to duplicately managed state.");
            }
        })));

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
    /// #[rocket::launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Launch Message", |_| {
    ///             println!("Rocket is launching!");
    ///         }))
    /// }
    /// ```
    #[inline]
    pub fn attach<F: Fairing>(mut self, fairing: F) -> Self {
        self.manifest.push(PreLaunchOp::Attach(Box::new(fairing)));
        self
    }

    /// Access the current state of this `Rocket` instance.
    ///
    /// The `Cargo` type provides methods such as [`Cargo::routes()`]
    /// and [`Cargo::state()`]. This method is called to get a `Cargo`
    /// instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite();
    /// let config = rocket.inspect().await.config();
    /// # let _ = config;
    /// # });
    /// ```
    pub async fn inspect(&mut self) -> &Cargo {
        self.actualize_manifest().await;
        self.cargo()
    }

    /// Returns `Some` of the managed state value for the type `T` if it is
    /// being managed by `self`. Otherwise, returns `None`.
    ///
    /// This function is equivalent to `.inspect().await.state()` and is
    /// provided as a convenience.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[derive(PartialEq, Debug)]
    /// struct MyState(&'static str);
    ///
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite().manage(MyState("hello!"));
    /// assert_eq!(rocket.state::<MyState>().await, Some(&MyState("hello!")));
    /// # });
    /// ```
    pub async fn state<T: Send + Sync + 'static>(&mut self) -> Option<&T> {
        self.inspect().await.state()
    }

    /// Returns the active configuration.
    ///
    /// This function is equivalent to `.inspect().await.config()` and is
    /// provided as a convenience.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Rocket;
    /// use rocket::fairing::AdHoc;
    ///
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite();
    /// println!("Rocket config: {:?}", rocket.config().await);
    /// # });
    /// ```
    pub async fn config(&mut self) -> &Config {
        self.inspect().await.config()
    }

    /// Returns a [`ShutdownHandle`], which can be used to gracefully terminate
    /// the instance of Rocket. In routes, you should use the [`ShutdownHandle`]
    /// request guard.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::{thread, time::Duration};
    /// #
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite();
    /// let handle = rocket.inspect().await.shutdown_handle();
    ///
    /// # if false {
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(10));
    ///     handle.shutdown();
    /// });
    ///
    /// // Shuts down after 10 seconds
    /// let shutdown_result = rocket.launch().await;
    /// assert!(shutdown_result.is_ok());
    /// # }
    /// # });
    /// ```
    #[inline(always)]
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown_handle.clone()
    }

    /// Perform "pre-launch" checks: verify that there are no routing colisions
    /// and that there were no fairing failures.
    pub(crate) async fn prelaunch_check(&mut self) -> Result<(), LaunchError> {
        self.actualize_manifest().await;
        if let Err(e) = self.router.collisions() {
            return Err(LaunchError::new(LaunchErrorKind::Collision(e)));
        }

        if let Some(failures) = self.fairings.failures() {
            return Err(LaunchError::new(LaunchErrorKind::FailedFairings(failures.to_vec())))
        }

        Ok(())
    }

    /// Returns a `Future` that drives the server, listening for and dispatching
    /// requests to mounted routes and catchers. The `Future` completes when the
    /// server is shut down via a [`ShutdownHandle`], encounters a fatal error,
    /// or if the the `ctrlc` configuration option is set, when `Ctrl+C` is
    /// pressed.
    ///
    /// # Error
    ///
    /// If there is a problem starting the application, an [`Error`] is
    /// returned. Note that a value of type `Error` panics if dropped
    /// without first being inspected. See the [`Error`] documentation for
    /// more information.
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
    pub async fn launch(mut self) -> Result<(), crate::error::Error> {
        use std::net::ToSocketAddrs;
        use futures::future::Either;
        use crate::error::Error::Launch;

        self.prelaunch_check().await.map_err(crate::error::Error::Launch)?;

        let full_addr = format!("{}:{}", self.config.address, self.config.port);
        let addr = match full_addr.to_socket_addrs() {
            Ok(mut addrs) => addrs.next().expect(">= 1 socket addr"),
            Err(e) => return Err(Launch(e.into())),
        };

        // FIXME: Make `ctrlc` a known `Rocket` config option.
        // If `ctrl-c` shutdown is enabled, we `select` on `the ctrl-c` signal
        // and server. Otherwise, we only wait on the `server`, hence `pending`.
        let shutdown_handle = self.shutdown_handle.clone();
        let shutdown_signal = match self.config.get_bool("ctrlc") {
            Ok(false) => futures::future::pending().boxed(),
            _ => tokio::signal::ctrl_c().boxed(),
        };

        let server = {
            macro_rules! listen_on {
                ($expr:expr) => {{
                    let listener = match $expr {
                        Ok(ok) => ok,
                        Err(err) => return Err(Launch(LaunchError::new(LaunchErrorKind::Bind(err))))
                    };
                    self.listen_on(listener)
                }};
            }

            #[cfg(feature = "tls")] {
                if let Some(tls) = self.config.tls.clone() {
                    listen_on!(crate::http::tls::bind_tls(addr, tls.certs, tls.key).await).boxed()
                } else {
                    listen_on!(crate::http::private::bind_tcp(addr).await).boxed()
                }
            }
            #[cfg(not(feature = "tls"))] {
                listen_on!(crate::http::private::bind_tcp(addr).await).boxed()
            }
        };

        match futures::future::select(shutdown_signal, server).await {
            Either::Left((Ok(()), server)) => {
                // Ctrl-was pressed. Signal shutdown, wait for the server.
                shutdown_handle.shutdown();
                server.await
            }
            Either::Left((Err(err), server)) => {
                // Error setting up ctrl-c signal. Let the user know.
                warn!("Failed to enable `ctrl+c` graceful signal shutdown.");
                info_!("Error: {}", err);
                server.await
            }
            // Server shut down before Ctrl-C; return the result.
            Either::Right((result, _)) => result,
        }
    }
}

impl std::fmt::Debug for PreLaunchOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) ->  std::fmt::Result {
        use PreLaunchOp::*;
        match self {
            Mount(origin, routes) => f.debug_tuple("PreLaunchOp::Mount")
                .field(&origin)
                .field(&routes)
                .finish(),
            Register(catchers) => f.debug_tuple("PreLaunchOp::Register")
                .field(&catchers)
                .finish(),
            Manage(name, _) => f.debug_tuple("PreLaunchOp::Manage")
                .field(&name)
                .finish(),
            Attach(fairing) => f.debug_tuple("PreLaunchOp::Attach")
                .field(&fairing.info())
                .finish()
        }
    }
}

impl std::ops::Deref for Cargo {
    type Target = Rocket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Cargo {
    /// Returns an iterator over all of the routes mounted on this instance of
    /// Rocket.
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
    /// # rocket::async_test(async {
    ///     let mut rocket = rocket::ignite()
    ///         .mount("/", routes![hello])
    ///         .mount("/hi", routes![hello]);
    ///
    ///     for route in rocket.inspect().await.routes() {
    ///         match route.base() {
    ///             "/" => assert_eq!(route.uri.path(), "/hello"),
    ///             "/hi" => assert_eq!(route.uri.path(), "/hi/hello"),
    ///             _ => unreachable!("only /hello, /hi/hello are expected")
    ///         }
    ///     }
    ///
    ///     assert_eq!(rocket.inspect().await.routes().count(), 2);
    /// # });
    /// }
    /// ```
    #[inline(always)]
    pub fn routes(&self) -> impl Iterator<Item = &Route> + '_ {
        self.0.router.routes()
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
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite().manage(MyState("hello!"));
    ///
    /// let cargo = rocket.inspect().await;
    /// assert_eq!(cargo.state::<MyState>(), Some(&MyState("hello!")));
    /// # });
    /// ```
    #[inline(always)]
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.0.managed_state.try_get()
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
    /// #[rocket::launch]
    /// fn rocket() -> rocket::Rocket {
    ///     rocket::ignite()
    ///         .attach(AdHoc::on_launch("Config Printer", |cargo| {
    ///             println!("Rocket launch config: {:?}", cargo.config());
    ///         }))
    /// }
    /// ```
    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.0.config
    }
}
