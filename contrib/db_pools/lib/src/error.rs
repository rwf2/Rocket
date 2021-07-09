use std::fmt;

/// A general error type for use by [`Pool`](crate::Pool#implementing)
/// implementors and returned by the [`Connection`](crate::Connection) request
/// guard.
#[derive(Debug)]
pub enum Error<A, B = A> {
    /// An error that occured during database/pool initialization.
    Init(A),

    /// An error that ocurred while retrieving a connection from the pool.
    Get(B),

    /// A [`Figment`](crate::figment::Figment) configuration error.
    Config(crate::figment::Error),
}

impl<A: fmt::Display, B: fmt::Display> fmt::Display for Error<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Init(e) => write!(f, "failed to initialize database: {}", e),
            Error::Get(e) => write!(f, "failed to get db connection: {}", e),
            Error::Config(e) => write!(f, "bad configuration: {}", e),
        }
    }
}

impl<A, B> std::error::Error for Error<A, B>
    where A: fmt::Debug + fmt::Display, B: fmt::Debug + fmt::Display {}

impl<A, B> From<crate::figment::Error> for Error<A, B> {
    fn from(e: crate::figment::Error) -> Self {
        Self::Config(e)
    }
}
