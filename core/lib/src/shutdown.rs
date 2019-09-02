use crate::request::{FromRequest, Outcome, Request};
use futures::channel::mpsc;

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
pub struct ShutdownHandle(crate mpsc::Sender<()>);

impl ShutdownHandle {
    /// Notify Rocket to shut down gracefully.
    #[inline]
    pub fn shutdown(mut self) {
        // Intentionally ignore any error, as the only scenarios this can happen
        // is sending too many shutdown requests or we're already shut down.
        let _ = self.0.try_send(());
    }
}

impl FromRequest<'_, '_> for ShutdownHandle {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_request(request: &Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.state.managed.get::<ShutdownHandleManaged>().0.clone())
    }
}

// Use this type in managed state to avoid placing `ShutdownHandle` in it.
crate struct ShutdownHandleManaged(crate ShutdownHandle);
