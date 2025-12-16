use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{FutureExt, StreamExt};

use crate::request::{FromRequest, Outcome, Request};
use crate::shutdown::{ShutdownConfig, TripWire};

/// A request guard and future for graceful shutdown.
///
/// A server shutdown is manually requested by calling [`Shutdown::notify()`]
/// or, if enabled, through [automatic triggers] like `Ctrl-C`. Rocket will stop
/// accepting new requests, finish handling any pending requests, wait a grace
/// period before cancelling any outstanding I/O, and return `Ok()` to the
/// caller of [`Rocket::launch()`]. Graceful shutdown is configured via
/// [`ShutdownConfig`](crate::config::ShutdownConfig).
///
/// [`Rocket::launch()`]: crate::Rocket::launch()
/// [automatic triggers]: crate::shutdown::Shutdown#triggers
///
/// # Detecting Shutdown
///
/// `Shutdown` is also a future that resolves when [`Shutdown::notify()`] is
/// called. This can be used to detect shutdown in any part of the application:
///
/// ```rust
/// # extern crate rocket_community as rocket;
///
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
/// # #[macro_use] extern crate rocket_community as rocket;
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
pub struct Shutdown {
    wire: TripWire,
}

#[derive(Debug, Clone)]
pub struct Stages {
    pub start: Shutdown,
    pub grace: Shutdown,
    pub mercy: Shutdown,
}

impl Shutdown {
    fn new() -> Self {
        Shutdown {
            wire: TripWire::new(),
        }
    }

    /// Notify the application to shut down gracefully.
    ///
    /// This function returns immediately; pending requests will continue to run
    /// until completion or expiration of the grace period, which ever comes
    /// first, before the actual shutdown occurs. The grace period can be
    /// configured via [`ShutdownConfig`]'s `grace` field.
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    ///
    /// # use rocket::*;
    /// use rocket::Shutdown;
    ///
    /// #[get("/shutdown")]
    /// fn shutdown(shutdown: Shutdown) -> &'static str {
    ///     shutdown.notify();
    ///     "Shutting down..."
    /// }
    /// ```
    #[inline(always)]
    pub fn notify(&self) {
        self.wire.trip();
    }

    /// Returns `true` if `Shutdown::notify()` has already been called.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    ///
    /// # use rocket::*;
    /// use rocket::Shutdown;
    ///
    /// #[get("/shutdown")]
    /// fn shutdown(shutdown: Shutdown) {
    ///     shutdown.notify();
    ///     assert!(shutdown.notified());
    /// }
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn notified(&self) -> bool {
        self.wire.tripped()
    }
}

impl Future for Shutdown {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.wire.poll_unpin(cx)
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

impl Stages {
    pub fn new() -> Self {
        Stages {
            start: Shutdown::new(),
            grace: Shutdown::new(),
            mercy: Shutdown::new(),
        }
    }

    pub(crate) fn spawn_listener(&self, config: &ShutdownConfig) {
        use futures::future::{select, Either};
        use futures::stream;

        let mut signal = match config.signal_stream() {
            Some(stream) => Either::Left(stream.chain(stream::pending())),
            None => Either::Right(stream::pending()),
        };

        let start = self.start.clone();
        let (grace, grace_duration) = (self.grace.clone(), config.grace());
        let (mercy, mercy_duration) = (self.mercy.clone(), config.mercy());
        tokio::spawn(async move {
            if let Either::Left((sig, start)) = select(signal.next(), start).await {
                warn!("Received {}. Shutdown started.", sig.unwrap());
                start.notify();
            }

            tokio::time::sleep(grace_duration).await;
            warn!("Shutdown grace period elapsed. Shutting down I/O.");
            grace.notify();

            tokio::time::sleep(mercy_duration).await;
            warn!("Mercy period elapsed. Terminating I/O.");
            mercy.notify();
        });
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
