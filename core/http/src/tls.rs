pub use tokio_rustls::TlsAcceptor;
pub use tokio_rustls::rustls;

pub use rustls::internal::pemfile;
pub use rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig};

// TODO.async: extract from hyper-sync-rustls some convenience
// functions to load certs and keys
