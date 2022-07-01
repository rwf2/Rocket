//! Support for mutual TLS client certificates.
//!
//! For details on how to configure mutual TLS, see
//! [`MutualTls`](crate::config::MutualTls) and the [TLS
//! guide](https://rocket.rs/v0.5-rc/guide/configuration/#tls). See
//! [`Certificate`] for a request guard that validated, verifies, and retrieves
//! client certificates.

#[doc(inline)]
pub use crate::http::tls::mtls::*;

use crate::request::{Request, FromRequest, Outcome};
use crate::outcome::{try_outcome, IntoOutcome};
use crate::http::Status;

#[crate::async_trait]
impl<'r> FromRequest<'r> for Certificate<'r> {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let certs = try_outcome!(req.connection.client_certificates.as_ref().or_forward(()));
        let data = try_outcome!(certs.chain_data().or_forward(()));
        Certificate::parse(data).into_outcome(Status::Unauthorized)
    }
}
