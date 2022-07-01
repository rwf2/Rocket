use crate::{Request, Response};
use crate::http::Status;

/// Type alias for the return type of a [`Catcher`](crate::Catcher)'s
/// [`Handler::handle()`].
pub type Result<'r> = std::result::Result<Response<'r>, crate::http::Status>;

/// Type alias for the return type of a _raw_ [`Catcher`](crate::Catcher)'s
/// [`Handler`].
pub type BoxFuture<'r, T = Result<'r>> = futures::future::BoxFuture<'r, T>;

/// Trait implemented by [`Catcher`](crate::Catcher) error handlers.
///
/// This trait is exactly like a [`Route`](crate::Route)'s
/// [`Handler`](crate::route::Handler) except it handles errors instead of
/// requests. Thus, the documentation for
/// [`route::Handler`](crate::route::Handler) applies to this trait as well. We
/// defer to it for full details.
///
/// ## Async Trait
///
/// This is an _async_ trait. Implementations must be decorated
/// [`#[rocket::async_trait]`](crate::async_trait).
///
/// # Example
///
/// Say you'd like to write a handler that changes its functionality based on a
/// `Kind` enum value that the user provides. Such a handler might be written
/// and used as follows:
///
/// ```rust,no_run
/// use rocket::{Request, Catcher, catcher};
/// use rocket::response::{Response, Responder};
/// use rocket::http::Status;
///
/// #[derive(Copy, Clone)]
/// enum Kind {
///     Simple,
///     Intermediate,
///     Complex,
/// }
///
/// #[derive(Clone)]
/// struct CustomHandler(Kind);
///
/// #[rocket::async_trait]
/// impl catcher::Handler for CustomHandler {
///     async fn handle<'r>(&self, status: Status, req: &'r Request<'_>) -> catcher::Result<'r> {
///         let inner = match self.0 {
///             Kind::Simple => "simple".respond_to(req)?,
///             Kind::Intermediate => "intermediate".respond_to(req)?,
///             Kind::Complex => "complex".respond_to(req)?,
///         };
///
///         Response::build_from(inner).status(status).ok()
///     }
/// }
///
/// impl CustomHandler {
///     /// Returns a `default` catcher that uses `CustomHandler`.
///     fn default(kind: Kind) -> Vec<Catcher> {
///         vec![Catcher::new(None, CustomHandler(kind))]
///     }
///
///     /// Returns a catcher for code `status` that uses `CustomHandler`.
///     fn catch(status: Status, kind: Kind) -> Vec<Catcher> {
///         vec![Catcher::new(status.code, CustomHandler(kind))]
///     }
/// }
///
/// #[rocket::launch]
/// fn rocket() -> _ {
///     rocket::build()
///         // to handle only `404`
///         .register("/", CustomHandler::catch(Status::NotFound, Kind::Simple))
///         // or to register as the default
///         .register("/", CustomHandler::default(Kind::Simple))
/// }
/// ```
///
/// Note the following:
///
///   1. `CustomHandler` implements `Clone`. This is required so that
///      `CustomHandler` implements `Cloneable` automatically. The `Cloneable`
///      trait serves no other purpose but to ensure that every `Handler`
///      can be cloned, allowing `Catcher`s to be cloned.
///   2. `CustomHandler`'s methods return `Vec<Route>`, allowing for use
///      directly as the parameter to `rocket.register("/", )`.
///   3. Unlike static-function-based handlers, this custom handler can make use
///      of internal state.
#[crate::async_trait]
pub trait Handler: Cloneable + Send + Sync + 'static {
    /// Called by Rocket when an error with `status` for a given `Request`
    /// should be handled by this handler.
    ///
    /// Error handlers _should not_ fail and thus _should_ always return `Ok`.
    /// Nevertheless, failure is allowed, both for convenience and necessity. If
    /// an error handler fails, Rocket's default `500` catcher is invoked. If it
    /// succeeds, the returned `Response` is used to respond to the client.
    async fn handle<'r>(&self, status: Status, req: &'r Request<'_>) -> Result<'r>;
}

// We write this manually to avoid double-boxing.
impl<F: Clone + Sync + Send + 'static> Handler for F
    where for<'x> F: Fn(Status, &'x Request<'_>) -> BoxFuture<'x>,
{
    fn handle<'r, 'life0, 'life1, 'async_trait>(
        &'life0 self,
        status: Status,
        req: &'r Request<'life1>,
    ) -> BoxFuture<'r>
        where 'r: 'async_trait,
              'life0: 'async_trait,
              'life1: 'async_trait,
              Self: 'async_trait,
    {
        self(status, req)
    }
}

#[cfg(test)]
pub fn dummy_handler<'r>(_: Status, _: &'r Request<'_>) -> BoxFuture<'r> {
   Box::pin(async move { Ok(Response::new()) })
}

mod private {
    pub trait Sealed {}
    impl<T: super::Handler + Clone> Sealed for T {}
}

/// Helper trait to make a [`Catcher`](crate::Catcher)'s `Box<dyn Handler>`
/// `Clone`.
///
/// This trait cannot be implemented directly. Instead, implement `Clone` and
/// [`Handler`]; all types that implement `Clone` and `Handler` automatically
/// implement `Cloneable`.
pub trait Cloneable: private::Sealed {
    #[doc(hidden)]
    fn clone_handler(&self) -> Box<dyn Handler>;
}

impl<T: Handler + Clone> Cloneable for T {
    fn clone_handler(&self) -> Box<dyn Handler> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Handler> {
    fn clone(&self) -> Box<dyn Handler> {
        self.clone_handler()
    }
}
