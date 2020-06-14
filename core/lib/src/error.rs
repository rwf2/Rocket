//! Types representing various errors that can occur in a Rocket application.

use std::{io, fmt};
use std::sync::atomic::{Ordering, AtomicBool};

use yansi::Paint;

use crate::router::Route;

/// An error that occurs when running a Rocket server.
///
/// Errors can happen immediately upon launch ([`LaunchError`])
/// or more rarely during the server's execution.
#[derive(Debug)]
pub enum Error {
    Launch(LaunchError),
    Run(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Launch(e) => write!(f, "Rocket failed to launch: {}", e),
            Error::Run(e) => write!(f, "error while running server: {}", e),
        }
    }
}

impl std::error::Error for Error { }

/// The kind of launch error that occurred.
///
/// In almost every instance, a launch error occurs because of an I/O error;
/// this is represented by the `Io` variant. A launch error may also occur
/// because of ill-defined routes that lead to collisions or because a fairing
/// encountered an error; these are represented by the `Collision` and
/// `FailedFairing` variants, respectively.
#[derive(Debug)]
pub enum LaunchErrorKind {
    /// Binding to the provided address/port failed.
    Bind(io::Error),
    /// An I/O error occurred during launch.
    Io(io::Error),
    /// Route collisions were detected.
    Collision(Vec<(Route, Route)>),
    /// A launch fairing reported an error.
    FailedFairings(Vec<&'static str>),
}

/// An error that occurs during launch.
///
/// A `LaunchError` is returned by [`launch()`](crate::Rocket::launch()) when
/// launching an application fails.
///
/// # Panics
///
/// A value of this type panics if it is dropped without first being inspected.
/// An _inspection_ occurs when any method is called. For instance, if
/// `println!("Error: {}", e)` is called, where `e: LaunchError`, the
/// `Display::fmt` method being called by `println!` results in `e` being marked
/// as inspected; a subsequent `drop` of the value will _not_ result in a panic.
/// The following snippet illustrates this:
///
/// ```rust
/// use rocket::error::Error;
///
/// # let _ = async {
/// if let Err(error) = rocket::ignite().launch().await {
///     match error {
///         Error::Launch(error) => {
///             // This case is only reached if launching failed. This println "inspects" the error.
///             println!("Launch failed! Error: {}", error);
///
///             // This call to drop (explicit here for demonstration) will do nothing.
///             drop(error);
///         }
///         Error::Run(error) => {
///             // This case is reached if launching succeeds, but the server had a fatal error later
///             println!("Server failed! Error: {}", error);
///         }
///     }
/// }
///
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
/// A `LaunchError` value should usually be allowed to `drop` without
/// inspection. There are two exceptions to this suggestion.
///
///   1. If you are writing a library or high-level application on-top of
///      Rocket, you likely want to inspect the value before it drops to avoid a
///      Rocket-specific `panic!`. This typically means simply printing the
///      value.
///
///   2. You want to display your own error messages.
pub struct LaunchError {
    handled: AtomicBool,
    kind: LaunchErrorKind
}

impl LaunchError {
    #[inline(always)]
    pub(crate) fn new(kind: LaunchErrorKind) -> LaunchError {
        LaunchError { handled: AtomicBool::new(false), kind }
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
    /// use rocket::error::Error;
    /// # let _ = async {
    /// if let Err(error) = rocket::ignite().launch().await {
    ///     match error {
    ///         Error::Launch(err) => println!("Found a launch error: {}", err.kind()),
    ///         Error::Run(err) => println!("Error at runtime"),
    ///     }
    /// }
    /// # };
    /// ```
    #[inline]
    pub fn kind(&self) -> &LaunchErrorKind {
        self.mark_handled();
        &self.kind
    }
}

impl From<io::Error> for LaunchError {
    #[inline]
    fn from(error: io::Error) -> LaunchError {
        LaunchError::new(LaunchErrorKind::Io(error))
    }
}

impl fmt::Display for LaunchErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LaunchErrorKind::Bind(ref e) => write!(f, "binding failed: {}", e),
            LaunchErrorKind::Io(ref e) => write!(f, "I/O error: {}", e),
            LaunchErrorKind::Collision(_) => write!(f, "route collisions detected"),
            LaunchErrorKind::FailedFairings(_) => write!(f, "a launch fairing failed"),
        }
    }
}

impl fmt::Debug for LaunchError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mark_handled();
        self.kind().fmt(f)
    }
}

impl fmt::Display for LaunchError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mark_handled();
        write!(f, "{}", self.kind())
    }
}

impl Drop for LaunchError {
    fn drop(&mut self) {
        if self.was_handled() {
            return
        }

        match *self.kind() {
            LaunchErrorKind::Bind(ref e) => {
                error!("Rocket failed to bind network socket to given address/port.");
                panic!("{}", e);
            }
            LaunchErrorKind::Io(ref e) => {
                error!("Rocket failed to launch due to an I/O error.");
                panic!("{}", e);
            }
            LaunchErrorKind::Collision(ref collisions) => {
                error!("Rocket failed to launch due to the following routing collisions:");
                for &(ref a, ref b) in collisions {
                    info_!("{} {} {}", a, Paint::red("collides with").italic(), b)
                }

                info_!("Note: Collisions can usually be resolved by ranking routes.");
                panic!("route collisions detected");
            }
            LaunchErrorKind::FailedFairings(ref failures) => {
                error!("Rocket failed to launch due to failing fairings:");
                for fairing in failures {
                    info_!("{}", fairing);
                }

                panic!("launch fairing failure");
            }
        }
    }
}

use crate::http::uri;
use crate::http::ext::IntoOwned;
use crate::http::route::{Error as SegmentError};

/// Error returned by [`set_uri()`](crate::Route::set_uri()) on invalid URIs.
#[derive(Debug)]
pub enum RouteUriError {
    /// The base (mount point) or route path contains invalid segments.
    Segment,
    /// The route URI is not a valid URI.
    Uri(uri::Error<'static>),
    /// The base (mount point) contains dynamic segments.
    DynamicBase,
}

impl<'a> From<(&'a str, SegmentError<'a>)> for RouteUriError {
    fn from(_: (&'a str, SegmentError<'a>)) -> Self {
        RouteUriError::Segment
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
            RouteUriError::Segment => {
                write!(f, "The URI contains malformed dynamic route path segments.")
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
