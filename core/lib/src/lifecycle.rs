use futures::future::{Future, FutureExt};

use crate::data::IoHandler;
use crate::form::Form;
use crate::http::{Header, Method, Status};
use crate::outcome::Outcome;
use crate::trace::Trace;
use crate::util::Formatter;
use crate::{catcher, route, Data, Orbit, Request, Response, Rocket};

// A token returned to force the execution of one method before another.
pub(crate) struct RequestToken;

async fn catch_handle<Fut, T, F>(name: Option<&str>, run: F) -> Option<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    macro_rules! panic_info {
        ($name:expr, $e:expr) => {{
            error!(
                handler = name.as_ref().map(display),
                "handler panicked\n\
                This is an application bug.\n\
                A panic in Rust must be treated as an exceptional event.\n\
                Panicking is not a suitable error handling mechanism.\n\
                Unwinding, the result of a panic, is an expensive operation.\n\
                Panics will degrade application performance.\n\
                Instead of panicking, return `Option` and/or `Result`.\n\
                Values of either type can be returned directly from handlers.\n\
                A panic is treated as an internal server error."
            );

            $e
        }};
    }

    let run = std::panic::AssertUnwindSafe(run);
    #[allow(clippy::manual_inspect)]
    let fut = std::panic::catch_unwind(run)
        .map_err(|e| panic_info!(name, e))
        .ok()?;

    #[allow(clippy::manual_inspect)]
    std::panic::AssertUnwindSafe(fut)
        .catch_unwind()
        .await
        .map_err(|e| panic_info!(name, e))
        .ok()
}

impl Rocket<Orbit> {
    /// Preprocess the request for Rocket things. Currently, this means:
    ///
    ///   * Rewriting the method in the request if _method form field exists.
    ///   * Run the request fairings.
    ///
    /// This is the only place during lifecycle processing that `Request` is
    /// mutable. Keep this in-sync with the `FromForm` derive.
    pub(crate) async fn preprocess(
        &self,
        req: &mut Request<'_>,
        data: &mut Data<'_>,
    ) -> RequestToken {
        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        if req.method() == Method::Post && req.content_type().is_some_and(|v| v.is_form()) {
            let peek_buffer = data.peek(32).await;
            let method = std::str::from_utf8(peek_buffer)
                .ok()
                .and_then(|raw_form| Form::values(raw_form).next())
                .filter(|field| field.name == "_method")
                .and_then(|field| field.value.parse().ok());

            if let Some(method) = method {
                req.set_method(method);
            }
        }

        // Run request fairings.
        self.fairings.handle_request(req, data).await;

        RequestToken
    }

    /// Dispatches the request to the router and processes the outcome to
    /// produce a response. If the initial outcome is a *forward* and the
    /// request was a HEAD request, the request is rewritten and rerouted as a
    /// GET. This is automatic HEAD handling.
    ///
    /// After performing the above, if the outcome is a forward or error, the
    /// appropriate error catcher is invoked to produce the response. Otherwise,
    /// the successful response is used directly.
    ///
    /// Finally, new cookies in the cookie jar are added to the response,
    /// Rocket-specific headers are written, and response fairings are run. Note
    /// that error responses have special cookie handling. See `handle_error`.
    pub(crate) async fn dispatch<'r, 's: 'r>(
        &'s self,
        _token: RequestToken,
        request: &'r Request<'s>,
        data: Data<'r>,
        // io_stream: impl Future<Output = io::Result<IoStream>> + Send,
    ) -> Response<'r> {
        // Remember if the request is `HEAD` for later body stripping.
        let was_head_request = request.method() == Method::Head;

        // Route the request and run the user's handlers.
        let mut response = match self.route(request, data).await {
            Outcome::Success(response) => response,
            Outcome::Forward((data, _)) if request.method() == Method::Head => {
                tracing::Span::current().record("autohandled", true);

                // Dispatch the request again with Method `GET`.
                request._set_method(Method::Get);
                match self.route(request, data).await {
                    Outcome::Success(response) => response,
                    Outcome::Error(status) => self.dispatch_error(status, request).await,
                    Outcome::Forward((_, status)) => self.dispatch_error(status, request).await,
                }
            }
            Outcome::Forward((_, status)) => self.dispatch_error(status, request).await,
            Outcome::Error(status) => self.dispatch_error(status, request).await,
        };

        // Set the cookies. Note that error responses will only include cookies
        // set by the error handler. See `handle_error` for more.
        let delta_jar = request.cookies().take_delta_jar();
        for cookie in delta_jar.delta() {
            response.adjoin_header(cookie);
        }

        // Add a default 'Server' header if it isn't already there.
        // TODO: If removing Hyper, write out `Date` header too.
        if let Some(ident) = request.rocket().config.ident.as_str() {
            if !response.headers().contains("Server") {
                response.set_header(Header::new("Server", ident));
            }
        }

        // Run the response fairings.
        self.fairings.handle_response(request, &mut response).await;

        // Strip the body if this is a `HEAD` request or a 304 response.
        if was_head_request || response.status() == Status::NotModified {
            response.strip_body();
        }

        // If the response status is 204, strip the body and its size (no
        // content-length header). Otherwise, check if the body is sized and use
        // that size to set the content-length headr appropriately.
        if response.status() == Status::NoContent {
            *response.body_mut() = crate::response::Body::unsized_none();
        } else if let Some(size) = response.body_mut().size().await {
            response.set_raw_header("Content-Length", size.to_string());
        }

        if let Some(alt_svc) = request.rocket().alt_svc() {
            response.set_raw_header("Alt-Svc", alt_svc);
        }

        // TODO: Should upgrades be handled here? We miss them on local clients.
        response
    }

    pub(crate) fn extract_io_handler<'r>(
        request: &'r Request<'_>,
        response: &mut Response<'r>,
        // io_stream: impl Future<Output = io::Result<IoStream>> + Send,
    ) -> Option<(String, Box<dyn IoHandler + 'r>)> {
        let upgrades = request.headers().get("upgrade");
        let Ok(upgrade) = response.search_upgrades(upgrades) else {
            info!(
                upgrades = %Formatter(|f| f.debug_list()
                    .entries(request.headers().get("upgrade"))
                    .finish()),
                "request wants upgrade but no i/o handler matched\n\
                refusing to upgrade request"
            );

            return None;
        };

        if let Some((proto, io_handler)) = upgrade {
            let proto = proto.to_string();
            response.set_status(Status::SwitchingProtocols);
            response.set_raw_header("Connection", "Upgrade");
            response.set_raw_header("Upgrade", proto.clone());
            return Some((proto, io_handler));
        }

        None
    }

    /// Calls the handler for each matching route until one of the handlers
    /// returns success or error, or there are no additional routes to try, in
    /// which case a `Forward` with the last forwarding state is returned.
    #[inline]
    #[tracing::instrument("routing", skip_all, fields(
        method = %request.method(),
        uri = %request.uri(),
        format = request.format().map(display),
    ))]
    async fn route<'s, 'r: 's>(
        &'s self,
        request: &'r Request<'s>,
        mut data: Data<'r>,
    ) -> route::Outcome<'r> {
        // Go through all matching routes until we fail or succeed or run out of
        // routes to try, in which case we forward with the last status.
        let mut status = Status::NotFound;
        for route in self.router.route(request) {
            // Retrieve and set the requests parameters.
            route.trace_info();
            request.set_route(route);

            let name = route.name.as_deref();
            let outcome = catch_handle(name, || route.handler.handle(request, data))
                .await
                .unwrap_or(Outcome::Error(Status::InternalServerError));

            // Check if the request processing completed (Some) or if the
            // request needs to be forwarded. If it does, continue the loop
            outcome.trace_info();
            match outcome {
                o @ Outcome::Success(_) | o @ Outcome::Error(_) => return o,
                Outcome::Forward(forwarded) => (data, status) = forwarded,
            }
        }

        Outcome::Forward((data, status))
    }

    // Invokes the catcher for `status`. Returns the response on success.
    //
    // Resets the cookie jar delta state to prevent any modifications from
    // earlier unsuccessful paths from being reflected in the error response.
    //
    // On catcher error, the 500 error catcher is attempted. If _that_ errors,
    // the (infallible) default 500 error cather is used.
    #[tracing::instrument("catching", skip_all, fields(status = status.code, uri = %req.uri()))]
    pub(crate) async fn dispatch_error<'r, 's: 'r>(
        &'s self,
        mut status: Status,
        req: &'r Request<'s>,
    ) -> Response<'r> {
        // We may wish to relax this in the future.
        req.cookies().reset_delta();

        loop {
            // Dispatch to the `status` catcher.
            match self.invoke_catcher(status, req).await {
                Ok(r) => return r,
                // If the catcher failed, try `500` catcher, unless this is it.
                Err(e) if status.code != 500 => {
                    warn!(
                        status = e.map(|r| r.code),
                        "catcher failed: trying 500 catcher"
                    );
                    status = Status::InternalServerError;
                }
                // The 500 catcher failed. There's no recourse. Use default.
                Err(e) => {
                    error!(status = e.map(|r| r.code), "500 catcher failed");
                    return catcher::default_handler(Status::InternalServerError, req);
                }
            }
        }
    }

    /// Invokes the handler with `req` for catcher with status `status`.
    ///
    /// In order of preference, invoked handler is:
    ///   * the user's registered handler for `status`
    ///   * the user's registered `default` handler
    ///   * Rocket's default handler for `status`
    ///
    /// Return `Ok(result)` if the handler succeeded. Returns `Ok(Some(Status))`
    /// if the handler ran to completion but failed. Returns `Ok(None)` if the
    /// handler panicked while executing.
    async fn invoke_catcher<'s, 'r: 's>(
        &'s self,
        status: Status,
        req: &'r Request<'s>,
    ) -> Result<Response<'r>, Option<Status>> {
        if let Some(catcher) = self.router.catch(status, req) {
            catcher.trace_info();
            catch_handle(catcher.name.as_deref(), || {
                catcher.handler.handle(status, req)
            })
            .await
            .map(|result| result.map_err(Some))
            .unwrap_or_else(|| Err(None))
        } else {
            info!(name: "catcher", name = "rocket::default", "uri.base" = "/", code = status.code,
                "no registered catcher: using Rocket default");
            Ok(catcher::default_handler(status, req))
        }
    }
}
