pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum KeyError {
    BadKeyCount(usize),
    Io(std::io::Error),
    Unsupported(rustls::Error),
    BadItem(rustls_pemfile::Item),
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Bind(Box<dyn std::error::Error + Send + 'static>),
    Tls(rustls::Error),
    Mtls(rustls::server::VerifierBuilderError),
    CertChain(std::io::Error),
    PrivKey(KeyError),
    CertAuth(rustls::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            Io(e) => write!(f, "i/o error during tls binding: {e}"),
            Tls(e) => write!(f, "tls configuration error: {e}"),
            Mtls(e) => write!(f, "mtls verifier error: {e}"),
            CertChain(e) => write!(f, "failed to process certificate chain: {e}"),
            PrivKey(e) => write!(f, "failed to process private key: {e}"),
            CertAuth(e) => write!(f, "failed to process certificate authority: {e}"),
            Bind(e) => write!(f, "failed to bind to network interface: {e}"),
        }
    }
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use KeyError::*;

        match self {
            Io(e) => write!(f, "error reading key file: {e}"),
            BadKeyCount(0) => write!(f, "no valid keys found. is the file malformed?"),
            BadKeyCount(n) => write!(f, "expected exactly 1 key, found {n}"),
            Unsupported(e) => write!(f, "key is valid but is unsupported: {e}"),
            BadItem(i) => write!(f, "found unexpected item in key file: {i:#?}"),
        }
    }
}

impl std::error::Error for KeyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KeyError::Io(e) => Some(e),
            KeyError::Unsupported(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Tls(e) => Some(e),
            Error::Mtls(e) => Some(e),
            Error::CertChain(e) => Some(e),
            Error::PrivKey(e) => Some(e),
            Error::CertAuth(e) => Some(e),
            Error::Bind(e) => Some(&**e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
       Error::Io(e)
    }
}

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Self {
        Error::Tls(e)
    }
}

impl From<rustls::server::VerifierBuilderError> for Error {
    fn from(value: rustls::server::VerifierBuilderError) -> Self {
        Error::Mtls(value)
    }
}

impl From<KeyError> for Error {
    fn from(value: KeyError) -> Self {
        Error::PrivKey(value)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(v: std::convert::Infallible) -> Self {
        v.into()
    }
}
