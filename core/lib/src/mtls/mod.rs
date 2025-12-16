//! Support for mutual TLS client certificates.
//!
//! For details on how to configure mutual TLS, see [`MtlsConfig`] and the [TLS
//! guide](https://rocket.rs/master/guide/configuration/#tls). See
//! [`Certificate`] for a request guard that validates, verifies, and retrieves
//! client certificates.

pub mod oid {
    //! Lower-level OID types re-exported from
    //! [`oid_registry`](https://docs.rs/oid-registry/0.4) and
    //! [`der-parser`](https://docs.rs/der-parser/7).

    pub use x509_parser::objects::*;
    pub use x509_parser::oid_registry::*;
}

pub mod bigint {
    //! Signed and unsigned big integer types re-exported from
    //! [`num_bigint`](https://docs.rs/num-bigint/0.4).
    pub use x509_parser::der_parser::num_bigint::*;
}

pub mod x509 {
    //! Lower-level X.509 types re-exported from
    //! [`x509_parser`](https://docs.rs/x509-parser/0.13).
    //!
    //! Lack of documentation is directly inherited from the source crate.
    //! Prefer to use Rocket's wrappers when possible.

    pub use x509_parser::prelude::*;
}

mod certificate;
mod config;
mod error;
mod name;

pub use certificate::{Certificate, CertificateDer};
pub use config::MtlsConfig;
pub use error::Error;
pub use name::Name;

/// A type alias for `Result` with the error type set to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
