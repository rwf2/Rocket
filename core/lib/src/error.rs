//! Types representing various errors that can occur in a Rocket application.

use std::{io, fmt};
use std::sync::atomic::{Ordering, AtomicBool};

use yansi::Paint;

use crate::router::Route;

/// An error that occurs during launch.
///
/// An `Error` is returned by [`launch()`](crate::Rocket::launch()) when
/// launching an application fails or, more rarely, when the runtime fails after
/// lauching.
///
/// # Panics
///
/// A value of this type panics if it is dropped without first being inspected.
/// An _inspection_ occurs when any method is called. For instance, if
/// `println!("Error: {}", e)` is called, where `e: Error`, the `Display::fmt`
/// method being called by `println!` results in `e` being marked as inspected;
/// a subsequent `drop` of the value will _not_ result in a panic. The following
/// snippet illustrates this:
///
/// ```rust
/// # let _ = async {
/// if let Err(error) = rocket::ignite().launch().await {
///     // This println "inspects" the error.
///     println!("Launch failed! Error: {}", error);
///
///     // This call to drop (explicit here for demonstration) will do nothing.
///     drop(error);
/// }
/// # };
/// ```
///
/// When a value of this type panics, the corresponding error message is pretty
/// printed to the console. The following illustrates this:
///
/// ```rust
/// # let _ = async {
/// let error = rocket::ignite().launch().await;
///
/// // This call to drop (explicit here for demonstration) will result in
/// // `error` being pretty-printed to the console along with a `panic!`.
/// drop(error);
/// # };
/// ```
///
/// # Usage
///
/// An `Error` value should usually be allowed to `drop` without inspection.
/// There are at least two exceptions:
///
///   1. If you are writing a library or high-level application on-top of
///      Rocket, you likely want to inspect the value before it drops to avoid a
///      Rocket-specific `panic!`. This typically means simply printing the
///      value.
///
///   2. You want to display your own error messages.
pub struct Error {
    handled: AtomicBool,
    kind: ErrorKind
}

/// The kind error that occurred.
///
/// In almost every instance, a launch error occurs because of an I/O error;
/// this is represented by the `Io` variant. A launch error may also occur
/// because of ill-defined routes that lead to collisions or because a fairing
/// encountered an error; these are represented by the `Collision` and
/// `FailedFairing` variants, respectively.
#[derive(Debug)]
pub enum ErrorKind {
    /// Binding to the provided address/port failed.
    Bind(io::Error),
    /// An I/O error occurred during launch.
    Io(io::Error),
    /// An I/O error occurred in the runtime.
    Runtime(Box<dyn std::error::Error + Send + Sync>),
    /// Route collisions were detected.
    Collision(Vec<(Route, Route)>),
    /// A launch fairing reported an error.
    FailedFairings(Vec<&'static str>),
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

impl Error {
    #[inline(always)]
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error { handled: AtomicBool::new(false), kind }
    }

    #[inline(always)]
    fn was_handled(&self) -> bool {
        self.handled.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn mark_handled(&self) {
        self.handled.store(true, Ordering::Release)
    }

    /// Retrieve the `kind` of the launch error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::error::ErrorKind;
    ///
    /// # let _ = async {
    /// if let Err(error) = rocket::ignite().launch().await {
    ///     match error.kind() {
    ///         ErrorKind::Io(e) => println!("found an i/o launch error: {}", e),
    ///         e => println!("something else happened: {}", e)
    ///     }
    /// }
    /// # };
    /// ```
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        self.mark_handled();
        &self.kind
    }
}

impl std::error::Error for Error {  }

impl fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Bind(e) => write!(f, "binding failed: {}", e),
            ErrorKind::Io(e) => write!(f, "I/O error: {}", e),
            ErrorKind::Collision(_) => write!(f, "route collisions detected"),
            ErrorKind::FailedFairings(_) => write!(f, "a launch fairing failed"),
            ErrorKind::Runtime(e) => write!(f, "runtime error: {}", e)
        }
    }
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mark_handled();
        self.kind().fmt(f)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mark_handled();
        write!(f, "{}", self.kind())
    }
}

impl Drop for Error {
    fn drop(&mut self) {
        if self.was_handled() {
            return
        }

        match *self.kind() {
            ErrorKind::Bind(ref error) => {
                error_span!("bind_error", "Rocket failed to bind network socket to given address/port.").in_scope(|| {
                    info!(%error);
                });
                panic!("aborting due to binding o error");
            }
            ErrorKind::Io(ref error) => {
                error_span!("io_error", "Rocket failed to launch due to an I/O error.").in_scope(|| {
                    info!(%error);
                });
                panic!("aborting due to i/o error");
            }
            ErrorKind::Collision(ref collisions) => {
                error_span!("collisions", "Rocket failed to launch due to the following routing collisions:").in_scope(|| {
                    for &(ref a, ref b) in collisions {
                        info!("{} {} {}", a, Paint::red("collides with").italic(), b);
                    }

                    info!("Note: Collisions can usually be resolved by ranking routes.");
                });
                panic!("route collisions detected");
            }
            ErrorKind::FailedFairings(ref failures) => {
                error_span!("fairing_error", "Rocket failed to launch due to failing fairings:").in_scope(|| {
                    for fairing in failures {
                        info!(%fairing);
                    }
                });
                panic!("aborting due to launch fairing failure");
            }
            ErrorKind::Runtime(ref error) => {
                error_span!("runtime_error", "An error occured in the runtime:").in_scope(|| {
                    info!(%error);
                });
                panic!("aborting due to runtime failure");
            }
        }
    }
}

use crate::http::uri;
use crate::http::ext::IntoOwned;
use crate::http::route::Error as SegmentError;

/// Error returned by [`Route::map_base()`] on invalid URIs.
#[derive(Debug)]
pub enum RouteUriError {
    /// The base (mount point) or route path contains invalid segments.
    Segment(String, String),
    /// The route URI is not a valid URI.
    Uri(uri::Error<'static>),
    /// The base (mount point) contains dynamic segments.
    DynamicBase,
}

impl<'a> From<(&'a str, SegmentError<'a>)> for RouteUriError {
    fn from((seg, err): (&'a str, SegmentError<'a>)) -> Self {
        RouteUriError::Segment(seg.into(), err.to_string())
    }
}

impl<'a> From<uri::Error<'a>> for RouteUriError {
    fn from(error: uri::Error<'a>) -> Self {
        RouteUriError::Uri(error.into_owned())
    }
}

impl fmt::Display for RouteUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteUriError::Segment(seg, err) => {
                write!(f, "Malformed segment '{}': {}", Paint::white(seg), err)
            }
            RouteUriError::DynamicBase => {
                write!(f, "The mount point contains dynamic parameters.")
            }
            RouteUriError::Uri(error) => {
                write!(f, "Malformed URI: {}", error)
            }
        }
    }
}
