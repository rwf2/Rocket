pub mod oid {
    //! Lower-level OID types re-exported from
    //! [`oid_registry`](https://docs.rs/oid-registry/0.1) and
    //! [`der-parser`](https://docs.rs/der-parser/5).

    pub use x509_parser::oid_registry::*;
    pub use x509_parser::der_parser::oid::*;
    pub use x509_parser::objects::*;
}

pub mod bigint {
    //! Signed and unsigned big integer types re-exported from
    //! [`num_bigint`](https://docs.rs/num-bigint/0.4).
    pub use x509_parser::der_parser::num_bigint::*;
}

pub mod x509 {
    //! Lower-level X.509 types re-exported from
    //! [`x509_parser`](https://docs.rs/x509-parser/0.9).
    //!
    //! Lack of documentation is directly inherited from the source crate.
    //! Prefer to use Rocket's wrappers when possible.

    pub use x509_parser::certificate::*;
    pub use x509_parser::cri_attributes::*;
    pub use x509_parser::error::*;
    pub use x509_parser::extensions::*;
    pub use x509_parser::revocation_list::*;
    pub use x509_parser::time::*;
    pub use x509_parser::x509::*;
    pub use x509_parser::der_parser::der;
    pub use x509_parser::der_parser::ber;
}

use std::fmt;
use std::ops::Deref;
use std::collections::HashMap;
use std::num::NonZeroUsize;

use ref_cast::RefCast;
use x509_parser::nom;
use x509::{ParsedExtension, X509Name, X509Certificate, TbsCertificate, X509Error};
use oid::OID_X509_EXT_SUBJECT_ALT_NAME as SUBJECT_ALT_NAME;

use crate::listener::RawCertificate;

/// A type alias for [`Result`](std::result::Result) with the error type set to
/// [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by the [`Certificate`] request guard.
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
    // FIXME: Waiting on https://github.com/rusticata/x509-parser/pull/92.
    // Parse(X509Error),
    /// An error occurred while parsing the certificate.
    #[doc(hidden)]
    Parse(String),
    /// The certificate parsed partially but is incomplete.
    ///
    /// If `Some(n)`, then `n` more bytes were expected. Otherwise, the number
    /// of expected bytes is unknown.
    Incomplete(Option<NonZeroUsize>),
    /// The certificate contained `.0` bytes of trailing data.
    Trailing(usize),
}

#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct Certificate<'a>(X509Certificate<'a>);

/// An X.509 Distinguished Name (DN) found in a [`Certificate`].
///
/// This type is a wrapper over [`x509::X509Name`] with convenient methods and
/// complete documentation. Should the data exposed by the inherent methods not
/// suffice, this type derefs to [`x509::X509Name`].
#[repr(transparent)]
#[derive(Debug, PartialEq, RefCast)]
pub struct Name<'a>(X509Name<'a>);

impl<'a> Certificate<'a> {
    fn parse_one(raw: &[u8]) -> Result<X509Certificate<'_>> {
        let (left, x509) = X509Certificate::from_der(raw)?;
        if !left.is_empty() {
            return Err(Error::Trailing(left.len()));
        }

        if x509.subject().as_raw().is_empty() {
            if let Some(ext) = x509.extensions().get(&SUBJECT_ALT_NAME) {
                if !matches!(ext.parsed_extension(), ParsedExtension::SubjectAlternativeName(..)) {
                    return Err(Error::NoSubject);
                } else if !ext.critical {
                    return Err(Error::NonCriticalSubjectAlt);
                }
            } else {
                return Err(Error::NoSubject);
            }
        }

        Ok(x509)
    }

    #[inline(always)]
    fn inner(&self) -> &TbsCertificate<'a> {
        &self.0.tbs_certificate
    }

    /// PRIVATE: For internal Rocket use only!
    #[doc(hidden)]
    pub fn parse(chain: &[RawCertificate]) -> Result<Certificate<'_>> {
        match chain.first() {
            Some(cert) => Certificate::parse_one(&cert.0).map(Certificate),
            None => Err(Error::Empty)
        }
    }

    pub fn serial(&self) -> &bigint::BigUint {
        &self.inner().serial
    }

    pub fn version(&self) -> u32 {
        self.inner().version.0
    }

    pub fn subject(&self) -> &Name<'a> {
        Name::ref_cast(&self.inner().subject)
    }

    pub fn issuer(&self) -> &Name<'a> {
        Name::ref_cast(&self.inner().issuer)
    }

    pub fn extensions(&self) -> &HashMap<oid::Oid<'a>, x509::X509Extension<'a>> {
        &self.inner().extensions
    }

    pub fn has_serial(&self, number: &str) -> Option<bool> {
        let uint: bigint::BigUint = number.parse().ok()?;
        Some(&uint == self.serial())
    }
}

impl<'a> Deref for Certificate<'a> {
    type Target = TbsCertificate<'a>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<'a> Name<'a> {
    pub fn common_name(&self) -> Option<&'a str> {
        self.common_names().next()
    }

    pub fn common_names(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.iter_by_oid(&oid::OID_X509_COMMON_NAME).filter_map(|n| n.as_str().ok())
    }

    pub fn email(&self) -> Option<&'a str> {
        self.emails().next()
    }

    pub fn emails(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.iter_by_oid(&oid::OID_PKCS9_EMAIL_ADDRESS).filter_map(|n| n.as_str().ok())
    }

    pub fn is_empty(&self) -> bool {
        self.0.as_raw().is_empty()
    }
}

impl<'a> Deref for Name<'a> {
    type Target = X509Name<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => write!(f, "parse failure: {}", e),
            Error::Incomplete(_) => write!(f, "incomplete certificate data"),
            Error::Trailing(n) => write!(f, "found {} trailing bytes", n),
            Error::Empty => write!(f, "empty certificate chain"),
            Error::NoSubject => write!(f, "empty subject without subjectAlt"),
            Error::NonCriticalSubjectAlt => write!(f, "empty subject without critical subjectAlt"),
        }
    }
}

impl From<nom::Err<X509Error>> for Error {
    fn from(e: nom::Err<X509Error>) -> Self {
        match e {
            nom::Err::Incomplete(nom::Needed::Unknown) => Error::Incomplete(None),
            nom::Err::Incomplete(nom::Needed::Size(n)) => Error::Incomplete(Some(n)),
            nom::Err::Error(e) | nom::Err::Failure(e) => Error::Parse(e.to_string()),
        }
    }
}

impl std::error::Error for Error {
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         Error::Parse(e) => Some(e),
    //         _ => None
    //     }
    // }
}
