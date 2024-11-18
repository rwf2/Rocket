use async_trait::async_trait;

use crate::http::Status;
use crate::outcome::Outcome;
use crate::request::FromRequest;
use crate::Request;

use crate::catcher::TypedError;

/// Trait used to extract types for an error catcher. You should
/// pretty much never implement this yourself. There are several
/// existing implementations, that should cover every need.
///
/// - [`Status`]: Extracts the HTTP status that this error is catching.
/// - [`&Request<'_>`]: Extracts a reference to the entire request that
///     triggered this error to begin with.
/// - [`T: FromRequest<'_>`]: Extracts type that implements `FromRequest`
/// - [`&dyn TypedError<'_>`]: Extracts the typed error, as a dynamic
///     trait object.
/// - [`Option<&dyn TypedError<'_>>`]: Same as previous, but succeeds even
///     if there is no typed error to extract.
///
/// [`Status`]: crate::http::Status
/// [`&Request<'_>`]: crate::request::Request
/// [`&dyn TypedError<'_>`]: crate::catcher::TypedError
/// [`Option<&dyn TypedError<'_>>`]: crate::catcher::TypedError
#[async_trait]
pub trait FromError<'r>: Sized {
    async fn from_error(
        status: Status,
        request: &'r Request<'r>,
        error: &'r dyn TypedError<'r>
    ) -> Result<Self, Status>;
}

#[async_trait]
impl<'r> FromError<'r> for Status {
    async fn from_error(
        status: Status,
        _r: &'r Request<'r>,
        _e: &'r dyn TypedError<'r>
    ) -> Result<Self, Status> {
        Ok(status)
    }
}

#[async_trait]
impl<'r> FromError<'r> for &'r Request<'r> {
    async fn from_error(
        _s: Status,
        req: &'r Request<'r>,
        _e: &'r dyn TypedError<'r>
    ) -> Result<Self, Status> {
        Ok(req)
    }
}

#[async_trait]
impl<'r, T: FromRequest<'r>> FromError<'r> for T {
    async fn from_error(
        _s: Status,
        req: &'r Request<'r>,
        _e: &'r dyn TypedError<'r>
    ) -> Result<Self, Status> {
        match T::from_request(req).await {
            Outcome::Success(val) => Ok(val),
            Outcome::Error(e) => {
                info!("Catcher guard error type: `{:?}`", e.name());
                Err(e.status())
            },
            Outcome::Forward(s) => {
                info!(status = %s, "Catcher guard forwarding");
                Err(s)
            },
        }
    }
}

#[async_trait]
impl<'r> FromError<'r> for &'r dyn TypedError<'r> {
    async fn from_error(
        _s: Status,
        _r: &'r Request<'r>,
        error: &'r dyn TypedError<'r>
    ) -> Result<Self, Status> {
        Ok(error)
    }
}
