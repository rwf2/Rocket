use std::io;
use std::sync::Arc;

use futures::stream::StreamExt;
use futures::future::{Future, BoxFuture};
use tokio::sync::oneshot;
use yansi::Paint;

use crate::Rocket;
use crate::handler;
use crate::request::{Request, FormItems};
use crate::data::Data;
use crate::response::{Body, Response};
use crate::outcome::Outcome;
use crate::error::{Error, ErrorKind};
use crate::logger::PaintExt;
use crate::ext::AsyncReadExt;

use crate::http::{Method, Status, Header, hyper};
use crate::http::private::{Listener, Connection, Incoming};
use crate::http::uri::Origin;

// A token returned to force the execution of one method before another.
pub(crate) struct Token;

// This function tries to hide all of the Hyper-ness from Rocket. It essentially
// converts Hyper types into Rocket types, then calls the `dispatch` function,
// which knows nothing about Hyper. Because responding depends on the
// `HyperResponse` type, this function does the actual response processing.
async fn hyper_service_fn(
    rocket: Arc<Rocket>,
    h_addr: std::net::SocketAddr,
    hyp_req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, io::Error> {
    // This future must return a hyper::Response, but the response body might
    // borrow from the request. Instead, write the body in another future that
    // sends the response metadata (and a body channel) prior.
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        // Get all of the information from Hyper.
        let (h_parts, h_body) = hyp_req.into_parts();

        // Convert the Hyper request into a Rocket request.
        let req_res = Request::from_hyp(
            &rocket, h_parts.method, h_parts.headers, &h_parts.uri, h_addr
        );

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
                return rocket.send_response(r, tx).await;
            }
        };

        // Retrieve the data from the hyper body.
        let mut data = Data::from_hyp(h_body).await;

        // Dispatch the request to get a response, then write that response out.
        let token = rocket.preprocess_request(&mut req, &mut data).await;
        let r = rocket.dispatch(token, &mut req, data).await;
        rocket.send_response(r, tx).await;
    });

    // Receive the response written to `tx` by the task above.
    rx.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

impl Rocket {
    /// Wrapper around `make_response` to log a success or failure.
    #[inline]
    async fn send_response(
        &self,
        response: Response<'_>,
        tx: oneshot::Sender<hyper::Response<hyper::Body>>,
    ) {
        match self.make_response(response, tx).await {
            Ok(()) => info_!("{}", Paint::green("Response succeeded.")),
            Err(e) => error_!("Failed to write response: {:?}.", e),
        }
    }

    /// Attempts to create a hyper response from `response` and send it to `tx`.
    #[inline]
    async fn make_response(
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

        let send_response = move |res: hyper::ResponseBuilder, body| -> io::Result<()> {
            let response = res.body(body)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            tx.send(response).map_err(|_| {
                let msg = "client disconnected before the response was started";
                io::Error::new(io::ErrorKind::BrokenPipe, msg)
            })
        };

        match response.body_mut() {
            None => {
                hyp_res = hyp_res.header(hyper::header::CONTENT_LENGTH, 0);
                send_response(hyp_res, hyper::Body::empty())?;
            }
            Some(body) => {
                if let Some(s) = body.size().await {
                    hyp_res = hyp_res.header(hyper::header::CONTENT_LENGTH, s);
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
        data: &mut Data
    ) -> Token {
        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        let (min_len, max_len) = ("_method=get".len(), "_method=delete".len());
        let peek_buffer = data.peek(max_len).await;
        let is_form = req.content_type().map_or(false, |ct| ct.is_form());

        if is_form && req.method() == Method::Post && peek_buffer.len() >= min_len {
            if let Ok(form) = std::str::from_utf8(peek_buffer) {
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
                        let try_next: BoxFuture<'_, _> =
                            Box::pin(self.route_and_process(request, data));
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
            let delta_jar = request.cookies().take_delta_jar();
            for cookie in delta_jar.delta() {
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
    #[inline]
    fn route<'s, 'r: 's>(
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
            let code = Paint::red(status.code);
            let response = if let Some(catcher) = self.catchers.get(&status.code) {
                catcher.handler.handle(status, req).await
            } else if let Some(ref default) =  self.default_catcher {
                warn_!("No {} catcher found. Using default catcher.", code);
                default.handler.handle(status, req).await
            } else {
                warn_!("No {} or default catcher found. Using Rocket default catcher.", code);
                crate::catcher::default(status, req)
            };

            // Dispatch to the catcher. If it fails, use the Rocket default 500.
            match response {
                Ok(r) => r,
                Err(err_status) => {
                    error_!("Catcher unexpectedly failed with {}.", err_status);
                    warn_!("Using Rocket's default 500 error catcher.");
                    let default = crate::catcher::default(Status::InternalServerError, req);
                    default.expect("Rocket has default 500 response")
                }
            }
        }
    }

    // TODO.async: Solidify the Listener APIs and make this function public
    pub(crate) async fn listen_on<L>(mut self, listener: L) -> Result<(), Error>
        where L: Listener + Send + Unpin + 'static,
              <L as Listener>::Connection: Send + Unpin + 'static,
    {
        // We do this twice if `listen_on` was called through `launch()` but
        // only once if `listen_on()` gets called directly.
        self.prelaunch_check().await?;

        // Freeze managed state for synchronization-free accesses later.
        self.managed_state.freeze();

        // Determine the address and port we actually bound to.
        self.config.port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
        let proto = self.config.tls.as_ref().map_or("http://", |_| "https://");
        let full_addr = format!("{}:{}", self.config.address, self.config.port);

        // Run the launch fairings.
        self.fairings.pretty_print_counts();
        self.fairings.handle_launch(&self);

        launch_info!("{}{} {}{}",
                     Paint::emoji("🚀 "),
                     Paint::default("Rocket has launched from").bold(),
                     Paint::default(proto).bold().underline(),
                     Paint::default(&full_addr).bold().underline());

        // Determine keep-alives.
        let http1_keepalive = self.config.keep_alive != 0;
        let http2_keep_alive = match self.config.keep_alive {
            0 => None,
            n => Some(std::time::Duration::from_secs(n as u64))
        };

        // We need to get this before moving `self` into an `Arc`.
        let mut shutdown_receiver = self.shutdown_receiver.take()
            .expect("shutdown receiver has already been used");

        let rocket = Arc::new(self);
        let service = hyper::make_service_fn(move |conn: &<L as Listener>::Connection| {
            let rocket = rocket.clone();
            let remote = conn.remote_addr().unwrap_or_else(|| ([0, 0, 0, 0], 0).into());
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service_fn(move |req| {
                    hyper_service_fn(rocket.clone(), remote, req)
                }))
            }
        });

        // NOTE: `hyper` uses `tokio::spawn()` as the default executor.
        hyper::Server::builder(Incoming::from_listener(listener))
            .http1_keepalive(http1_keepalive)
            .http2_keep_alive_interval(http2_keep_alive)
            .serve(service)
            .with_graceful_shutdown(async move { shutdown_receiver.recv().await; })
            .await
            .map_err(|e| Error::new(ErrorKind::Runtime(Box::new(e))))
    }
}
