use tls::Certificate;

#[derive(Debug)]
pub struct MutualTlsUser {
    peer_certs: Vec<Certificate>,
}

impl MutualTlsUser {
    pub fn new(peer_certs: Vec<Certificate>) -> MutualTlsUser {
        MutualTlsUser {
            peer_certs
        }
    }

    /// Get the common name
    pub fn name(&self) -> String {
        unimplemented!();
    }
}
