use crate::request::{FromRequest, Outcome, Request};
use tokio::sync::mpsc;

/// A `ShutdownHandle` can be used to instruct a Rocket server to gracefully
/// shut down. Once a server shutdown has been requested manually by calling
/// [`ShutdownHandle::shutdown()`] or automatically by `Ctrl-C` being pressed
/// (if enabled), Rocket will finish handling any pending requests and return to
/// the caller of [`Rocket::serve()`] or [`Rocket::launch()`].
///
/// [`Rocket::serve()`]: crate::Rocket::serve()
/// [`Rocket::launch()`]: crate::Rocket::launch()
///
/// # Example
///
/// ```rust,no_run
/// # #![feature(proc_macro_hygiene)]
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::shutdown::ShutdownHandle;
///
/// #[get("/shutdown")]
/// fn shutdown(handle: ShutdownHandle) -> &'static str {
///     handle.shutdown();
///     "Shutting down..."
/// }
///
/// #[rocket::main]
/// async fn main() {
///     let result = rocket::ignite()
///         .mount("/", routes![shutdown])
///         .launch()
///         .await;
///
///     // If the server shut down (by visting `/shutdown`), `result` is `Ok`.
///     result.expect("server failed unexpectedly");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ShutdownHandle(pub(crate) mpsc::Sender<()>);

impl ShutdownHandle {
    /// Notify Rocket to shut down gracefully. This function returns
    /// immediately; pending requests will continue to run until completion
    /// before the actual shutdown occurs.
    #[inline]
    pub fn shutdown(mut self) {
        // Intentionally ignore any error, as the only scenarios this can happen
        // is sending too many shutdown requests or we're already shut down.
        let _ = self.0.try_send(());
        info!("Server shutdown requested, waiting for all pending requests to finish.");
    }
}

#[crate::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for ShutdownHandle {
    type Error = std::convert::Infallible;

    #[inline]
    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.state.managed.get::<ShutdownHandleManaged>().0.clone())
    }
}

// Use this type in managed state to avoid placing `ShutdownHandle` in it.
pub(crate) struct ShutdownHandleManaged(pub ShutdownHandle);
