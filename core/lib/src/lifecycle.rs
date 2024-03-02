use yansi::Paint;
use futures::future::{FutureExt, Future};

use crate::{route, Rocket, Orbit, Request, Response, Data};
use crate::data::IoHandler;
use crate::http::{Method, Status, Header};
use crate::outcome::Outcome;
use crate::form::Form;

// A token returned to force the execution of one method before another.
pub(crate) struct RequestToken;

async fn catch_handle<Fut, T, F>(name: Option<&str>, run: F) -> Option<T>
    where F: FnOnce() -> Fut, Fut: Future<Output = T>,
{
    macro_rules! panic_info {
        ($name:expr, $e:expr) => {{
            match $name {
                Some(name) => error_!("Handler {} panicked.", name.primary()),
                None => error_!("A handler panicked.")
            };

            info_!("This is an application bug.");
            info_!("A panic in Rust must be treated as an exceptional event.");
            info_!("Panicking is not a suitable error handling mechanism.");
            info_!("Unwinding, the result of a panic, is an expensive operation.");
            info_!("Panics will degrade application performance.");
            info_!("Instead of panicking, return `Option` and/or `Result`.");
            info_!("Values of either type can be returned directly from handlers.");
            warn_!("A panic is treated as an internal server error.");
            $e
        }}
    }

    let run = std::panic::AssertUnwindSafe(run);
    let fut = std::panic::catch_unwind(move || run())
        .map_err(|e| panic_info!(name, e))
        .ok()?;

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
        data: &mut Data<'_>
    ) -> RequestToken {
        // Check if this is a form and if the form contains the special _method
        // field which we use to reinterpret the request's method.
        let (min_len, max_len) = ("_method=get".len(), "_method=delete".len());
        let peek_buffer = data.peek(max_len).await;
        let is_form = req.content_type().map_or(false, |ct| ct.is_form());

        if is_form && req.method() == Method::Post && peek_buffer.len() >= min_len {
            let method = std::str::from_utf8(peek_buffer).ok()
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
        info!("{}:", request);

        // Remember if the request is `HEAD` for later body stripping.
        let was_head_request = request.method() == Method::Head;

        // Route the request and run the user's handlers.
        let mut response = match self.route(request, data).await {
            Outcome::Success(response) => response,
            Outcome::Forward((data, _)) if request.method() == Method::Head => {
                info_!("Autohandling {} request.", "HEAD".primary().bold());

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

        // Strip the body if this is a `HEAD` request.
        if was_head_request {
            response.strip_body();
        }

        // TODO: Should upgrades be handled here? We miss them on local clients.
        response
    }

    pub(crate) fn extract_io_handler<'r>(
        request: &Request<'_>,
        response: &mut Response<'r>,
        // io_stream: impl Future<Output = io::Result<IoStream>> + Send,
    ) -> Option<Box<dyn IoHandler + 'r>> {
        let upgrades = request.headers().get("upgrade");
        let Ok(upgrade) = response.search_upgrades(upgrades) else {
            warn_!("Request wants upgrade but no I/O handler matched.");
            info_!("Request is not being upgraded.");
            return None;
        };

        if let Some((proto, io_handler)) = upgrade {
            info_!("Attempting upgrade with {proto} I/O handler.");
            response.set_status(Status::SwitchingProtocols);
            response.set_raw_header("Connection", "Upgrade");
            response.set_raw_header("Upgrade", proto.to_string());
            return Some(io_handler);
        }

        None
    }

    /// Calls the handler for each matching route until one of the handlers
    /// returns success or error, or there are no additional routes to try, in
    /// which case a `Forward` with the last forwarding state is returned.
    #[inline]
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
            info_!("Matched: {}", route);
            request.set_route(route);

            let name = route.name.as_deref();
            let outcome = catch_handle(name, || route.handler.handle(request, data)).await
                .unwrap_or(Outcome::Error(Status::InternalServerError));

            // Check if the request processing completed (Some) or if the
            // request needs to be forwarded. If it does, continue the loop
            // (None) to try again.
            info_!("{}", outcome.log_display());
            match outcome {
                o@Outcome::Success(_) | o@Outcome::Error(_) => return o,
                Outcome::Forward(forwarded) => (data, status) = forwarded,
            }
        }

        error_!("No matching routes for {}.", request);
        Outcome::Forward((data, status))
    }

    // Invokes the catcher for `status`. Returns the response on success.
    //
    // Resets the cookie jar delta state to prevent any modifications from
    // earlier unsuccessful paths from being reflected in the error response.
    //
    // On catcher error, the 500 error catcher is attempted. If _that_ errors,
    // the (infallible) default 500 error cather is used.
    pub(crate) async fn dispatch_error<'r, 's: 'r>(
        &'s self,
        mut status: Status,
        req: &'r Request<'s>
    ) -> Response<'r> {
        // We may wish to relax this in the future.
        req.cookies().reset_delta();

        // Dispatch to the `status` catcher.
        if let Ok(r) = self.invoke_catcher(status, req).await {
            return r;
        }

        // If it fails and it's not a 500, try the 500 catcher.
        if status != Status::InternalServerError {
            error_!("Catcher failed. Attempting 500 error catcher.");
            status = Status::InternalServerError;
            if let Ok(r) = self.invoke_catcher(status, req).await {
                return r;
            }
        }

        // If it failed again or if it was already a 500, use Rocket's default.
        error_!("{} catcher failed. Using Rocket default 500.", status.code);
        crate::catcher::default_handler(Status::InternalServerError, req)
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
        req: &'r Request<'s>
    ) -> Result<Response<'r>, Option<Status>> {
        if let Some(catcher) = self.router.catch(status, req) {
            warn_!("Responding with registered {} catcher.", catcher);
            let name = catcher.name.as_deref();
            catch_handle(name, || catcher.handler.handle(status, req)).await
                .map(|result| result.map_err(Some))
                .unwrap_or_else(|| Err(None))
        } else {
            let code = status.code.blue().bold();
            warn_!("No {} catcher registered. Using Rocket default.", code);
            Ok(crate::catcher::default_handler(status, req))
        }
    }

}
