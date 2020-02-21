use crate::request::{FromRequest, Outcome, Request};
use tokio::sync::broadcast;

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
/// ```rust
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
/// fn main() {
///     # if false {
///     rocket::ignite()
///         .mount("/", routes![shutdown])
///         .launch()
///         .expect("server failed unexpectedly");
///     # }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ShutdownHandle(pub(crate) broadcast::Sender<()>);

impl ShutdownHandle {
    /// Notify Rocket to shut down gracefully. This function returns
    /// immediately; pending requests will continue to run until completion
    /// before the actual shutdown occurs.
    #[inline]
    pub fn shutdown(self) {
        // Intentionally ignore any error, as the only scenarios this can happen
        // is sending too many shutdown requests or we're already shut down.
        let _ = self.0.send(());
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

/// A `ShutdownSignal` can be used to cooperatively shut down a Rocket server,
/// particularly to close a websocket or event stream.
///
/// # Example
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::shutdown::ShutdownSignal;
///
/// #[get("/wait-for-shutdown")]
/// async fn wait_for_shutdown(signal: ShutdownSignal) -> &'static str {
///     signal.wait().await;
///     "Server has shut down."
/// }
///
/// fn main() {
///     # if false {
///     rocket::ignite()
///         .mount("/", routes![wait_for_shutdown])
///         .launch()
///         .expect("server failed unexpectedly");
///     # }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ShutdownSignal(pub(crate) broadcast::Sender<()>);

impl ShutdownSignal {
    /// Wait for Rocket to shut down gracefully.
    #[inline]
    pub async fn wait(self) {
        // Intentionally ignore errors, as the only scenario where this can
        // happen is when we're already shutting down.
        let _ = self.0.subscribe().recv().await;
    }
}

#[crate::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for ShutdownSignal {
    type Error = std::convert::Infallible;

    #[inline]
    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Outcome::Success(ShutdownSignal(request.state.managed.get::<ShutdownHandleManaged>().0.clone().0))
    }
}
