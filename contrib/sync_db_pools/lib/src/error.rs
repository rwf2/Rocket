use rocket::figment;

/// A wrapper around `r2d2::Error`s or a custom database error type.
///
/// This type is only relevant to implementors of the [`Poolable`] trait. See
/// the [`Poolable`] documentation for more information on how to use this type.
///
/// [`Poolable`]: crate::Poolable
#[derive(Debug)]
pub enum Error<T> {
    /// A custom error of type `T`.
    Custom(T),
    /// An error occurred while initializing an `r2d2` pool.
    Pool(r2d2::Error),
    /// An error occurred while extracting a `figment` configuration.
    Config(figment::Error),
}

impl<T> From<figment::Error> for Error<T> {
    fn from(error: figment::Error) -> Self {
        Error::Config(error)
    }
}

impl<T> From<r2d2::Error> for Error<T> {
    fn from(error: r2d2::Error) -> Self {
        Error::Pool(error)
    }
}
