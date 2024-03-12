use std::{io, sync::Arc};

use rustls::{server::ClientHello, sign::{any_supported_type, CertifiedKey}};

use crate::tls::Config;
use crate::tls::util::{load_certs, load_private_key};

pub(crate) struct CertResolver(Arc<CertifiedKey>);
impl CertResolver {
    pub fn new<R>(config: &mut Config<R>) -> Result<Self, std::io::Error>
        where R: io::BufRead,
    { 
        let certs = load_certs(&mut config.cert_chain)?;
        let private_key = load_private_key(&mut config.private_key)?;
        let key = any_supported_type(&private_key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?;

        Ok(Self(Arc::new(CertifiedKey::new(certs, key))))
    }
}

impl rustls::server::ResolvesServerCert for CertResolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}
