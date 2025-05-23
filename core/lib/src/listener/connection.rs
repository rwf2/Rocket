use std::io;
use std::borrow::Cow;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::either::Either;

use super::Endpoint;

/// A collection of raw certificate data.
#[derive(Clone)]
pub struct Certificates<'r>(Cow<'r, [der::CertificateDer<'r>]>);

pub trait Connection: AsyncRead + AsyncWrite + Send + Unpin {
    fn endpoint(&self) -> io::Result<Endpoint>;

    /// DER-encoded X.509 certificate chain presented by the client, if any.
    ///
    /// The certificate order must be as it appears in the TLS protocol: the
    /// first certificate relates to the peer, the second certifies the first,
    /// the third certifies the second, and so on.
    ///
    /// Defaults to an empty vector to indicate that no certificates were
    /// presented.
    fn certificates(&self) -> Option<Certificates<'_>> { None }

    fn server_name(&self) -> Option<&str> { None }
}

impl<A: Connection, B: Connection> Connection for Either<A, B> {
    fn endpoint(&self) -> io::Result<Endpoint> {
        match self {
            Either::Left(c) => c.endpoint(),
            Either::Right(c) => c.endpoint(),
        }
    }

    fn certificates(&self) -> Option<Certificates<'_>> {
        match self {
            Either::Left(c) => c.certificates(),
            Either::Right(c) => c.certificates(),
        }
    }
}

impl Certificates<'_> {
    pub fn into_owned(self) -> Certificates<'static> {
        let cow = self.0.iter()
            .map(|der| der.clone().into_owned())
            .collect::<Vec<_>>()
            .into();

        Certificates(cow)
    }
}

#[cfg(feature = "mtls")]
#[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
mod der {
    use super::*;

    pub use crate::mtls::CertificateDer;

    impl<'r> Certificates<'r> {
        pub(crate) fn inner(&self) -> &[CertificateDer<'r>] {
            &self.0
        }
    }

    impl<'r> From<&'r [CertificateDer<'r>]> for Certificates<'r> {
        fn from(value: &'r [CertificateDer<'r>]) -> Self {
            Certificates(value.into())
        }
    }

    impl From<Vec<CertificateDer<'static>>> for Certificates<'static> {
        fn from(value: Vec<CertificateDer<'static>>) -> Self {
            Certificates(value.into())
        }
    }
}

#[cfg(not(feature = "mtls"))]
mod der {
    use std::marker::PhantomData;

    /// A thin wrapper over raw, DER-encoded X.509 client certificate data.
    #[derive(Clone)]
    pub struct CertificateDer<'r>(PhantomData<&'r [u8]>);

    impl CertificateDer<'_> {
        pub fn into_owned(self) -> CertificateDer<'static> {
            CertificateDer(PhantomData)
        }
    }
}
