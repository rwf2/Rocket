use std::fmt;
use std::num::NonZeroUsize;

use transient::Static;

use crate::{catcher::TypedError, mtls::x509::{self, nom}};
use crate::http::Status;

/// An error returned by the [`Certificate`](crate::mtls::Certificate) guard.
///
/// To retrieve this error in a handler, use an `mtls::Result<Certificate>`
/// guard type:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::get;
/// use rocket::mtls::{self, Certificate};
///
/// #[get("/auth")]
/// fn auth(cert: mtls::Result<Certificate<'_>>) {
///     match cert {
///         Ok(cert) => { /* do something with the client cert */ },
///         Err(e) => { /* do something with the error */ },
///     }
/// }
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    /// The certificate chain presented by the client had no certificates.
    Empty,
    /// The certificate contained neither a subject nor a subjectAlt extension.
    NoSubject,
    /// There is no subject and the subjectAlt is not marked as critical.
    NonCriticalSubjectAlt,
    /// An error occurred while parsing the certificate.
    Parse(x509::X509Error),
    /// The certificate parsed partially but is incomplete.
    ///
    /// If `Some(n)`, then `n` more bytes were expected. Otherwise, the number
    /// of expected bytes is unknown.
    Incomplete(Option<NonZeroUsize>),
    /// The certificate contained `.0` bytes of trailing data.
    Trailing(usize),
    /// The subject is not authorized
    SubjectUnauthorized,
}

impl Static for Error {}

impl<'r> TypedError<'r> for Error {
    fn status(&self) -> Status { Status::Unauthorized }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::Incomplete(_) => write!(f, "incomplete certificate data"),
            Error::Trailing(n) => write!(f, "found {} trailing bytes", n),
            Error::Empty => write!(f, "empty certificate chain"),
            Error::NoSubject => write!(f, "empty subject without subjectAlt"),
            Error::NonCriticalSubjectAlt => write!(f, "empty subject without critical subjectAlt"),
            Error::SubjectUnauthorized => write!(f, "subject not permitted"),
        }
    }
}

impl From<nom::Err<x509::X509Error>> for Error {
    fn from(e: nom::Err<x509::X509Error>) -> Self {
        match e {
            nom::Err::Incomplete(nom::Needed::Unknown) => Error::Incomplete(None),
            nom::Err::Incomplete(nom::Needed::Size(n)) => Error::Incomplete(Some(n)),
            nom::Err::Error(e) | nom::Err::Failure(e) => Error::Parse(e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(e) => Some(e),
            _ => None
        }
    }
}
