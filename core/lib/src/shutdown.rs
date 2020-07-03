use crate::request::{FromRequest, Outcome, Request};
use tokio::sync::broadcast;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
pub struct Shutdown(pub(crate) broadcast::Sender<()>, pub(crate) Arc<AtomicBool>);

impl Shutdown {
    /// Notify Rocket to shut down gracefully. This function returns
    /// immediately; pending requests will continue to run until completion
    /// before the actual shutdown occurs.
    #[inline]
    pub fn shutdown(self) {
        self.1.store(true, Ordering::SeqCst);
        // Intentionally ignore any error, as the only scenarios this can happen
        // is sending too many shutdown requests or we're already shut down.
        let _ = self.0.send(());
        info!("Server shutdown requested, waiting for all pending requests to finish.");
    }
    pub async fn wait(self) {
        // This uses four events:
        //
        // * the store event
        // * the send event
        //
        // * the subscribe event
        // * the load event
        //
        // Since both pairs of events are happening in parallel threads (potentially), we need to worry about
        // all possible interleavings, but events within a single thread cannot be reordered (store comes before
        // send, while subscribe comes before load, always). For this to work, either we store before we load,
        // or we subscribe before we send.
        //
        // In a sequential ordering, either the store came first, or the subscribe did. If the store came
        // before the subscribe, then it must have also come before the load, which means that the load will
        // pick up on the atomic change. If the subscribe came before the store, then it also came before
        // the send, meaning that the broadcast channel will kick us out instead.
        let mut recv = self.0.subscribe();
        if !self.1.load(Ordering::SeqCst) {
            let _ = recv.recv().await;
        }
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
