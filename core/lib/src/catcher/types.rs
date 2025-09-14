use std::fmt;

use either::Either;
use transient::{Any, CanRecoverFrom, Downcast, Transience};
use crate::{http::Status, response::status::Custom, Request, Response};
#[doc(inline)]
pub use transient::{Static, Transient, TypeId, Inv, CanTranscendTo};

/// Polyfill for trait upcasting to [`Any`]
pub trait AsAny<Tr: Transience>: Any<Tr> + Sealed<Tr> {
    /// The actual upcast
    fn as_any(&self) -> &dyn Any<Tr>;
    /// convience typeid of the inner typeid
    fn trait_obj_typeid(&self) -> TypeId;
}

use sealed::Sealed;
mod sealed {
    use transient::{Any, Transience, Transient, TypeId};

    use super::AsAny;

    pub trait Sealed<Tr> {}
    impl<'r, Tr: Transience, T: Any<Tr>> Sealed<Tr> for T { }
    impl<'r, Tr: Transience, T: Any<Tr> + Transient> AsAny<Tr> for T {
        fn as_any(&self) -> &dyn Any<Tr> {
            self
        }
        fn trait_obj_typeid(&self) -> transient::TypeId {
            TypeId::of::<T>()
        }
    }
}

/// This is the core of typed catchers. If an error type (returned by
/// FromParam, FromRequest, FromForm, FromData, or Responder) implements
/// this trait, it can be caught by a typed catcher. This trait
/// can be derived.
pub trait TypedError<'r>: AsAny<Inv<'r>> + Send + Sync + 'r {
    /// Generates a default response for this type (or forwards to a default catcher)
    #[allow(unused_variables)]
    fn respond_to(&self, request: &'r Request<'_>) -> Result<Response<'r>, Status> {
        Err(self.status())
    }
    fn respond_to2(self: Box<Self>, request: &'r Request<'_>) -> Result<Response<'r>, Status> {
        Err(self.status())
    }

    /// A descriptive name of this error type. Defaults to the type name.
    fn name(&self) -> &'static str { std::any::type_name::<Self>() }

    /// The error that caused this error. Defaults to None. Each source
    /// should only be returned for one index - this method will be called
    /// with indicies starting with 0, and increasing until it returns None,
    /// or reaches 5.
    ///
    /// # Warning
    /// A typed catcher will not attempt to follow the source of an error
    /// more than 5 times.
    fn source(&'r self, _idx: usize) -> Option<&'r (dyn TypedError<'r> + 'r)> { None }

    /// Status code
    fn status(&self) -> Status { Status::InternalServerError }
}

// TODO: this is less useful, since impls should generally use `Status` instead.
impl<'r> TypedError<'r> for () {  }

impl<'r> TypedError<'r> for Status {
    fn respond_to(&self, _r: &'r Request<'_>) -> Result<Response<'r>, Status> {
        Err(*self)
    }

    fn name(&self) -> &'static str {
        "<Status>"
    }

    fn status(&self) -> Status {
        *self
    }
}

impl<'r> From<Status> for Box<dyn TypedError<'r> + 'r> {
    fn from(value: Status) -> Self {
        Box::new(value)
    }
}

impl From<Box<dyn TypedError<'_> + '_>> for Status {
    fn from(value: Box<dyn TypedError<'_> + '_>) -> Self {
        value.status()
    }
}

impl<'r, A: TypedError<'r> + Transient, B: TypedError<'r> + Transient> TypedError<'r> for (A, B)
    where A::Transience: CanTranscendTo<Inv<'r>>,
          B::Transience: CanTranscendTo<Inv<'r>>,
{
    fn respond_to(&self, request: &'r Request<'_>) -> Result<Response<'r>, Status> {
        self.0.respond_to(request).or_else(|_| self.1.respond_to(request))
    }

    fn name(&self) -> &'static str {
        // TODO: This should make it more clear that both `A` and `B` work, but
        // would likely require const concatenation.
        std::any::type_name::<(A, B)>()
    }

    fn source(&'r self, idx: usize) -> Option<&'r (dyn TypedError<'r> + 'r)> {
        match idx {
            0 => Some(&self.0),
            1 => Some(&self.1),
            _ => None,
        }
    }

    fn status(&self) -> Status {
        self.0.status()
    }
}

impl<'r, R: TypedError<'r> + Transient> TypedError<'r> for Custom<R>
    where R::Transience: CanTranscendTo<Inv<'r>>
{
    fn respond_to(&self, request: &'r Request<'_>) -> Result<Response<'r>, Status> {
        self.1.respond_to(request)
    }

    fn name(&self) -> &'static str {
        self.1.name()
    }

    fn source(&'r self, idx: usize) -> Option<&'r (dyn TypedError<'r> + 'r)> {
        if idx == 0 { Some(&self.1) } else { None }
    }

    fn status(&self) -> Status {
        self.0
    }
}

impl<'r> TypedError<'r> for std::convert::Infallible {  }

impl<'r> TypedError<'r> for std::io::Error {
    fn status(&self) -> Status {
        match self.kind() {
            std::io::ErrorKind::NotFound => Status::NotFound,
            std::io::ErrorKind::PermissionDenied => Status::Unauthorized,
            std::io::ErrorKind::AlreadyExists => Status::Conflict,
            std::io::ErrorKind::InvalidInput => Status::BadRequest,
            _ => Status::InternalServerError,
        }
    }
}

impl<'r> TypedError<'r> for std::num::ParseIntError {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r> TypedError<'r> for std::num::ParseFloatError {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r> TypedError<'r> for std::str::ParseBoolError {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r> TypedError<'r> for std::string::FromUtf8Error {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r> TypedError<'r> for std::net::AddrParseError {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r> TypedError<'r> for crate::http::uri::error::PathError {
    fn status(&self) -> Status { Status::BadRequest }
}

#[cfg(feature = "json")]
impl<'r> TypedError<'r> for serde_json::Error {
    fn status(&self) -> Status { Status::BadRequest }
}

#[cfg(feature = "msgpack")]
impl<'r> TypedError<'r> for rmp_serde::encode::Error { }

#[cfg(feature = "msgpack")]
impl<'r> TypedError<'r> for rmp_serde::decode::Error {
    fn status(&self) -> Status {
        match self {
            rmp_serde::decode::Error::InvalidDataRead(e)
                if e.kind() == std::io::ErrorKind::UnexpectedEof => Status::BadRequest,
            | rmp_serde::decode::Error::TypeMismatch(..)
            | rmp_serde::decode::Error::OutOfRange
            | rmp_serde::decode::Error::LengthMismatch(..) => Status::UnprocessableEntity,
            _ => Status::BadRequest,
        }
    }
}

#[cfg(feature = "uuid")]
impl<'r> TypedError<'r> for uuid_::Error {
    fn status(&self) -> Status { Status::BadRequest }
}

impl<'r, L, R> TypedError<'r> for Either<L, R>
    where L: TypedError<'r> + Transient,
          L::Transience: CanTranscendTo<Inv<'r>>,
          R: TypedError<'r> + Transient,
          R::Transience: CanTranscendTo<Inv<'r>>,
{
    fn respond_to(&self, request: &'r Request<'_>) -> Result<Response<'r>, Status> {
        match self {
            Self::Left(v) => v.respond_to(request),
            Self::Right(v) => v.respond_to(request),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Left(v) => v.name(),
            Self::Right(v) => v.name(),
        }
    }

    fn source(&'r self, idx: usize) -> Option<&'r (dyn TypedError<'r> + 'r)> {
        if idx == 0 {
            match self {
                Self::Left(v) => Some(v),
                Self::Right(v) => Some(v),
            }
        } else {
            None
        }
    }

    fn status(&self) -> Status {
        match self {
            Self::Left(v) => v.status(),
            Self::Right(v) => v.status(),
        }
    }
}

impl fmt::Debug for dyn TypedError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} as TypedError>", self.name())
    }
}

/// Validates that a type implements `TypedError`. Used by the `#[catch]` attribute to ensure
/// the `TypeError` is first in the diagnostics.
#[doc(hidden)]
pub fn type_id_of<'r, T: TypedError<'r> + Transient + 'r>() -> (TypeId, &'static str) {
    (TypeId::of::<T>(), std::any::type_name::<T>())
}

/// Downcast an error type to the underlying concrete type. Used by the `#[catch]` attribute.
#[doc(hidden)]
pub fn downcast<'r, T>(v: &'r dyn TypedError<'r>) -> Option<&'r T>
    where T: TypedError<'r> + Transient + 'r,
          T::Transience: CanRecoverFrom<Inv<'r>>,
{
    // crate::trace::error!("Downcasting error from {}", v.name());
    v.as_any().downcast_ref()
}
