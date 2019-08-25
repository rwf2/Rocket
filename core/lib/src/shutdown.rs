use futures::{channel::mpsc, stream::StreamExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Wrapper around an mpsc channel.
#[derive(Debug)]
crate struct Shutdown {
    crate sender: ShutdownHandle,
    crate receiver: Option<Receiver>,
}

impl Shutdown {
    /// Create a `Shutdown`.
    #[inline]
    crate fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self {
            sender: ShutdownHandle(sender),
            receiver: Some(Receiver(receiver)),
        }
    }
}

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
///         .launch();
///     # }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ShutdownHandle(mpsc::Sender<()>);

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

/// Thin wrapper around an `mpsc::Receiver`,
/// implementing `Future`, which waits for the first value.
#[derive(Debug)]
crate struct Receiver(mpsc::Receiver<()>);

impl Future for Receiver {
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_next_unpin(cx).map(|opt| opt.unwrap_or(()))
    }
}
