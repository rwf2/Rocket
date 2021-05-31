use std::borrow::Cow;
use std::fmt;

/// A general error type designed for the `Poolable` trait.
///
/// [`Pool::initialize`] can return an error for any of several reasons:
///
///   * Missing or incorrect configuration, including some syntax errors
///   * An error connecting to the database.
///
/// [`Pool::initialize`]: crate::Pool::initialize
#[derive(Debug)]
pub enum Error<E> {
    /// An error in the configuration
    Config(Cow<'static, str>),

    /// A database-specific error occurred
    Db(E),
}

impl<E> Error<E> {
    /// Creates a new `Error` corresponding to an error in configuration.
    ///
    /// The message should indicate what field was incorrect and what was
    /// expected instead if applicable.
    pub fn config(message: impl Into<Cow<'static, str>>) -> Self {
        Self::Config(message.into())
    }
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Db(e) => e.fmt(f),
            Error::Config(e) => write!(f, "database connection pool configuration error: {}", e),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for Error<E> {}
