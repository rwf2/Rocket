//! Types representing various errors that can occur in a Rocket application.

use std::{io, fmt, process};
use std::error::Error as StdError;
use std::sync::Arc;

use figment::Profile;

use crate::listener::Endpoint;
use crate::{Catcher, Ignite, Orbit, Phase, Rocket, Route};
use crate::trace::Trace;

/// An error that occurred during launch or ignition.
///
/// An `Error` is returned by [`Rocket::launch()`] or [`Rocket::ignite()`] on
/// failure to launch or ignite, respectively. An `Error` may occur when the
/// configuration is invalid, when a route or catcher collision is detected, or
/// when a fairing fails to launch. An `Error` may also occur when the Rocket
/// instance fails to liftoff or when the Rocket instance fails to shutdown.
/// Finally, an `Error` may occur when a sentinel requests an abort.
///
/// To determine the kind of error that occurred, use [`Error::kind()`].
///
/// # Example
///
/// ```rust
/// # use rocket::*;
/// use rocket::trace::Trace;
/// use rocket::error::ErrorKind;
///
/// # async fn run() -> Result<(), rocket::error::Error> {
/// if let Err(e) = rocket::build().ignite().await {
///     match e.kind() {
///         ErrorKind::Bind(_, e) => info!("binding failed: {}", e),
///         ErrorKind::Io(e) => info!("I/O error: {}", e),
///         _ => e.trace_error(),
///     }
///
///     return Err(e);
/// }
/// # Ok(())
/// # }
/// ```
pub struct Error {
    pub(crate) kind: ErrorKind
}

/// The error kind that occurred. Returned by [`Error::kind()`].
///
/// In almost every instance, a launch error occurs because of an I/O error;
/// this is represented by the `Io` variant. A launch error may also occur
/// because of ill-defined routes that lead to collisions or because a fairing
/// encountered an error; these are represented by the `Collision` and
/// `FailedFairing` variants, respectively.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Binding to the network interface at `.0` (if known) failed with `.1`.
    Bind(Option<Endpoint>, Box<dyn StdError + Send>),
    /// An I/O error occurred during launch.
    Io(io::Error),
    /// A valid [`Config`](crate::Config) could not be extracted from the
    /// configured figment.
    Config(figment::Error),
    /// Route or catcher collisions were detected. At least one of `routes` or
    /// `catchers` is guaranteed to be non-empty.
    Collisions {
        /// Pairs of colliding routes, if any.
        routes: Vec<(Route, Route)>,
        /// Pairs of colliding catchers, if any.
        catchers: Vec<(Catcher, Catcher)>,
    },
    /// Launch fairing(s) failed.
    FailedFairings(Vec<crate::fairing::Info>),
    /// Sentinels requested abort.
    SentinelAborts(Vec<crate::sentinel::Sentry>),
    /// The configuration profile is not debug but no secret key is configured.
    InsecureSecretKey(Profile),
    /// Liftoff failed. Contains the Rocket instance that failed to shutdown.
    Liftoff(
        Result<Box<Rocket<Ignite>>, Arc<Rocket<Orbit>>>,
        tokio::task::JoinError,
    ),
    /// Shutdown failed. Contains the Rocket instance that failed to shutdown.
    Shutdown(Arc<Rocket<Orbit>>),
}

/// An error that occurs when a value was unexpectedly empty.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Empty;

/// An error that occurs when a value doesn't match one of the expected options.
///
/// This error is returned by the [`FromParam`] trait implementation generated
/// by the [`FromParam` derive](macro@rocket::FromParam) when the value of a
/// dynamic path segment does not match one of the expected variants. The
/// `value` field will contain the value that was provided, and `options` will
/// contain each of possible stringified variants.
///
/// [`FromParam`]: trait@rocket::request::FromParam
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::error::InvalidOption;
///
/// #[derive(FromParam)]
/// enum MyParam {
///     FirstOption,
///     SecondOption,
///     ThirdOption,
/// }
///
/// #[get("/<param>")]
/// fn hello(param: Result<MyParam, InvalidOption<'_>>) {
///     if let Err(e) = param {
///         assert_eq!(e.options, &["FirstOption", "SecondOption", "ThirdOption"]);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InvalidOption<'a> {
    /// The value that was provided.
    pub value: &'a str,
    /// The expected values: a slice of strings, one for each possible value.
    pub options: &'static [&'static str],
}

impl<'a> InvalidOption<'a> {
    #[doc(hidden)]
    pub fn new(value: &'a str, options: &'static [&'static str]) -> Self {
        Self { value, options }
    }
}

impl fmt::Display for InvalidOption<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unexpected value {:?}, expected one of {:?}", self.value, self.options)
    }
}

impl std::error::Error for InvalidOption<'_> {}

impl Error {
    #[inline(always)]
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }

    /// Returns the kind of error that occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::*;
    /// use rocket::trace::Trace;
    /// use rocket::error::ErrorKind;
    ///
    /// # async fn run() -> Result<(), rocket::error::Error> {
    /// if let Err(e) = rocket::build().ignite().await {
    ///     match e.kind() {
    ///         ErrorKind::Bind(_, e) => info!("binding failed: {}", e),
    ///         ErrorKind::Io(e) => info!("I/O error: {}", e),
    ///         _ => e.trace_error(),
    ///    }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Given the return value of [`Rocket::launch()`] or [`Rocket::ignite()`],
    /// which return a `Result<Rocket<P>, Error>`, logs the error, if any, and
    /// returns the appropriate exit code.
    ///
    /// For `Ok(_)`, returns `ExitCode::SUCCESS`. For `Err(e)`, logs the error
    /// and returns `ExitCode::FAILURE`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::*;
    /// use std::process::ExitCode;
    /// use rocket::error::Error;
    ///
    /// async fn run() -> ExitCode {
    ///     Error::report(rocket::build().launch().await)
    /// }
    /// ```
    pub fn report<P: Phase>(result: Result<Rocket<P>, Error>) -> process::ExitCode {
        match result {
            Ok(_) => process::ExitCode::SUCCESS,
            Err(e) => {
                span_error!("launch failure", "aborting launch due to error" => e.trace_error());
                process::ExitCode::SUCCESS
            }
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

impl From<figment::Error> for Error {
    fn from(e: figment::Error) -> Self {
        Error::new(ErrorKind::Config(e))
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::new(ErrorKind::Io(e))
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.kind {
            ErrorKind::Bind(_, e) => Some(&**e),
            ErrorKind::Io(e) => Some(e),
            ErrorKind::Collisions { .. } => None,
            ErrorKind::FailedFairings(_) => None,
            ErrorKind::InsecureSecretKey(_) => None,
            ErrorKind::Config(e) => Some(e),
            ErrorKind::SentinelAborts(_) => None,
            ErrorKind::Liftoff(_, e) => Some(e),
            ErrorKind::Shutdown(_) => None,
        }
    }
}

impl fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Bind(_, e) => write!(f, "binding failed: {e}"),
            ErrorKind::Io(e) => write!(f, "I/O error: {e}"),
            ErrorKind::Collisions { .. } => "collisions detected".fmt(f),
            ErrorKind::FailedFairings(_) => "launch fairing(s) failed".fmt(f),
            ErrorKind::InsecureSecretKey(_) => "insecure secret key config".fmt(f),
            ErrorKind::Config(_) => "failed to extract configuration".fmt(f),
            ErrorKind::SentinelAborts(_) => "sentinel(s) aborted".fmt(f),
            ErrorKind::Liftoff(_, _) => "liftoff failed".fmt(f),
            ErrorKind::Shutdown(_) => "shutdown failed".fmt(f),
        }
    }
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl fmt::Debug for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("empty parameter")
    }
}

impl fmt::Display for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("empty parameter")
    }
}

impl StdError for Empty { }

struct ServerError<'a>(&'a (dyn StdError + 'static));

impl fmt::Display for ServerError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error = &self.0;
        if let Some(e) = error.downcast_ref::<hyper::Error>() {
            write!(f, "request failed: {e}")?;
        } else if let Some(e) = error.downcast_ref::<io::Error>() {
            write!(f, "connection error: ")?;

            match e.kind() {
                io::ErrorKind::NotConnected => write!(f, "remote disconnected")?,
                io::ErrorKind::UnexpectedEof => write!(f, "remote sent early eof")?,
                io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted => write!(f, "terminated by remote")?,
                _ => write!(f, "{e}")?,
            }
        } else {
            write!(f, "http server error: {error}")?;
        }

        Ok(())
    }
}

/// Log an error that occurs during request processing
#[track_caller]
pub(crate) fn log_server_error(error: &(dyn StdError + 'static)) {
    let mut error: &(dyn StdError + 'static) = error;
    if error.downcast_ref::<hyper::Error>().is_some() {
        span_warn!("request error", "{}", ServerError(error) => {
            while let Some(source) = error.source() {
                error = source;
                warn!("{}", ServerError(error));
            }
        });
    } else {
        span_error!("server error", "{}", ServerError(error) => {
            while let Some(source) = error.source() {
                error = source;
                error!("{}", ServerError(error));
            }
        });
    }
}

#[doc(hidden)]
pub mod display_hack_impl {
    use super::*;
    use crate::util::Formatter;

    /// The *magic*.
    ///
    /// This type implements a `display()` method using an internal `T` that is
    /// either `fmt::Display` _or_ `fmt::Debug`, using the former when
    /// available. It does so by using a "specialization" hack: it has a blanket
    /// DefaultDisplay trait impl for all types that are `fmt::Debug` and a
    /// "specialized" inherent impl for all types that are `fmt::Display`.
    ///
    /// As long as `T: Display`, the "specialized" impl is what Rust will
    /// resolve `DisplayHack(v).display()` to when `T: fmt::Display` as it is an
    /// inherent impl. Otherwise, Rust will fall back to the blanket impl.
    pub struct DisplayHack<T: ?Sized>(pub T);

    pub trait DefaultDisplay {
        fn display(&self) -> impl fmt::Display;
    }

    /// Blanket implementation for `T: Debug`. This is what Rust will resolve
    /// `DisplayHack<T>::display` to when `T: Debug`.
    impl<T: fmt::Debug + ?Sized> DefaultDisplay for DisplayHack<T> {
        #[inline(always)]
        fn display(&self) -> impl fmt::Display {
            Formatter(|f| fmt::Debug::fmt(&self.0, f))
        }
    }

    /// "Specialized" implementation for `T: Display`. This is what Rust will
    /// resolve `DisplayHack<T>::display` to when `T: Display`.
    impl<T: fmt::Display + fmt::Debug + ?Sized> DisplayHack<T> {
        #[inline(always)]
        pub fn display(&self) -> impl fmt::Display + '_ {
            Formatter(|f| fmt::Display::fmt(&self.0, f))
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! display_hack {
    ($v:expr) => ({
        #[allow(unused_imports)]
        use $crate::error::display_hack_impl::{DisplayHack, DefaultDisplay as _};

        #[allow(unreachable_code)]
        DisplayHack($v).display()
    })
}

#[doc(hidden)]
pub use display_hack as display_hack;
