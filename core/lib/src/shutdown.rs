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
    ///
    /// This method only needs to be called once.
    #[inline]
    pub fn shutdown(&self) {
        // Intentionally ignore any error, as the only scenarios this can happen
        // is sending too many shutdown requests or we're already shut down.
        //
        // Clone to avoid requiring `self` to be mutable.
        let _ = self.0.clone().try_send(());
    }
}

impl FromRequest<'_, '_> for ShutdownHandle {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_request(request: &Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.state.managed.get::<ShutdownHandle>().clone())
    }
}
