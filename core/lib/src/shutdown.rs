use crate::request::{FromRequest, Outcome, Request};
use tokio::sync::mpsc;

/// A request guard to gracefully shutdown a Rocket server.
///
/// A server shutdown is manually requested by calling [`Shutdown::shutdown()`]
/// or, if enabled, by pressing `Ctrl-C`. Rocket will finish handling any
/// pending requests and return `Ok()` to the caller of [`Rocket::launch()`].
///
/// [`Rocket::launch()`]: crate::Rocket::launch()
///
/// # Example
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::Shutdown;
///
/// #[get("/shutdown")]
/// fn shutdown(handle: Shutdown) -> &'static str {
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
///     // If the server shut down (by visiting `/shutdown`), `result` is `Ok`.
///     result.expect("server failed unexpectedly");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Shutdown(pub(crate) mpsc::Sender<()>);

impl Shutdown {
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
impl<'a, 'r> FromRequest<'a, 'r> for Shutdown {
    type Error = std::convert::Infallible;

    #[inline]
    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.state.shutdown.clone())
    }
}
