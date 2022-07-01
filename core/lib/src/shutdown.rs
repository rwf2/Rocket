use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;

use futures::FutureExt;

use crate::request::{FromRequest, Outcome, Request};
use crate::trip_wire::TripWire;

/// A request guard and future for graceful shutdown.
///
/// A server shutdown is manually requested by calling [`Shutdown::notify()`]
/// or, if enabled, through [automatic triggers] like `Ctrl-C`. Rocket will stop accepting new
/// requests, finish handling any pending requests, wait a grace period before
/// cancelling any outstanding I/O, and return `Ok()` to the caller of
/// [`Rocket::launch()`]. Graceful shutdown is configured via
/// [`config::Shutdown`](crate::config::Shutdown).
///
/// [`Rocket::launch()`]: crate::Rocket::launch()
/// [automatic triggers]: crate::config::Shutdown#triggers
///
/// # Detecting Shutdown
///
/// `Shutdown` is also a future that resolves when [`Shutdown::notify()`] is
/// called. This can be used to detect shutdown in any part of the application:
///
/// ```rust
/// # use rocket::*;
/// use rocket::Shutdown;
///
/// #[get("/wait/for/shutdown")]
/// async fn wait_for_shutdown(shutdown: Shutdown) -> &'static str {
///     shutdown.await;
///     "Somewhere, shutdown was requested."
/// }
/// ```
///
/// See the [`stream`](crate::response::stream#graceful-shutdown) docs for an
/// example of detecting shutdown in an infinite responder.
///
/// Additionally, a completed shutdown request resolves the future returned from
/// [`Rocket::launch()`](crate::Rocket::launch()):
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::Shutdown;
///
/// #[get("/shutdown")]
/// fn shutdown(shutdown: Shutdown) -> &'static str {
///     shutdown.notify();
///     "Shutting down..."
/// }
///
/// #[rocket::main]
/// async fn main() {
///     let result = rocket::build()
///         .mount("/", routes![shutdown])
///         .launch()
///         .await;
///
///     // If the server shut down (by visiting `/shutdown`), `result` is `Ok`.
///     result.expect("server failed unexpectedly");
/// }
/// ```
#[derive(Debug, Clone)]
#[must_use = "`Shutdown` does nothing unless polled or `notify`ed"]
pub struct Shutdown(pub(crate) TripWire);

impl Shutdown {
    /// Notify the application to shut down gracefully.
    ///
    /// This function returns immediately; pending requests will continue to run
    /// until completion or expiration of the grace period, which ever comes
    /// first, before the actual shutdown occurs. The grace period can be
    /// configured via [`Shutdown::grace`](crate::config::Shutdown::grace).
    ///
    /// ```rust
    /// # use rocket::*;
    /// use rocket::Shutdown;
    ///
    /// #[get("/shutdown")]
    /// fn shutdown(shutdown: Shutdown) -> &'static str {
    ///     shutdown.notify();
    ///     "Shutting down..."
    /// }
    /// ```
    #[inline]
    pub fn notify(self) {
        self.0.trip();
    }
}

#[crate::async_trait]
impl<'r> FromRequest<'r> for Shutdown {
    type Error = std::convert::Infallible;

    #[inline]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.rocket().shutdown())
    }
}

impl Future for Shutdown {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::Shutdown;

    #[test]
    fn ensure_is_send_sync_clone_unpin() {
        fn is_send_sync_clone_unpin<T: Send + Sync + Clone + Unpin>() {}
        is_send_sync_clone_unpin::<Shutdown>();
    }
}
