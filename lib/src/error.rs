//! Types representing various errors that can occur in a Rocket application.

use std::{io, fmt};
use std::sync::atomic::{Ordering, AtomicBool};

use yansi::Paint;

use http::hyper;
use router::Route;

/// [unstable] Error type for Rocket. Likely to change.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    /// The request method was bad.
    BadMethod,
    /// The value could not be parsed.
    BadParse,
    /// There was no such route.
    NoRoute, // TODO: Add a chain of routes attempted.
    /// The error was internal.
    Internal,
    /// The requested key/index does not exist.
    NoKey,
}

/// The kind of launch error that occured.
///
/// In almost every instance, a launch error occurs because of an I/O error;
/// this is represented by the `Io` variant. A launch error may also occur
/// because of ill-defined routes that lead to collisions or because a fairing
/// encountered an error; these are represented by the `Collision` and
/// `FailedFairing` variants, respectively. The `Unknown` variant captures all
/// other kinds of launch errors.
#[derive(Debug)]
pub enum LaunchErrorKind {
    Io(io::Error),
    Collision(Vec<(Route, Route)>),
    FailedFairing,
    Unknown(Box<::std::error::Error + Send + Sync>)
}

/// An error that occurs during launch.
///
/// A `LaunchError` is returned by
/// [rocket::launch](/rocket/struct.Rocket.html#method.launch) when launching an
/// application fails for some reason.
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
/// # if false {
/// let error = rocket::ignite().launch();
///
/// // This line is only reached if launching failed. This "inspects" the error.
/// println!("Launch failed! Error: {}", error);
///
/// // This call to drop (explicit here for demonstration) will do nothing.
/// drop(error);
/// # }
/// ```
///
/// When a value of this type panics, the corresponding error message is pretty
/// printed to the console. The following illustrates this:
///
/// ```rust
/// # if false {
/// let error = rocket::ignite().launch();
///
/// // This call to drop (explicit here for demonstration) will result in
/// // `error` being pretty-printed to the console along with a `panic!`.
/// drop(error);
/// # }
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
    fn new(kind: LaunchErrorKind) -> LaunchError {
        LaunchError { handled: AtomicBool::new(false), kind: kind }
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
    /// # if false {
    /// let error = rocket::ignite().launch();
    ///
    /// // This line is only reached if launch failed.
    /// let error_kind = error.kind();
    /// # }
    /// ```
    #[inline]
    pub fn kind(&self) -> &LaunchErrorKind {
        self.mark_handled();
        &self.kind
    }
}

impl From<LaunchErrorKind> for LaunchError {
    #[inline]
    fn from(kind: LaunchErrorKind) -> LaunchError {
        LaunchError::new(kind)
    }
}

impl From<hyper::Error> for LaunchError {
    #[inline]
    fn from(error: hyper::Error) -> LaunchError {
        match error {
            hyper::Error::Io(e) => LaunchError::new(LaunchErrorKind::Io(e)),
            e => LaunchError::new(LaunchErrorKind::Unknown(Box::new(e)))
        }
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LaunchErrorKind::Io(ref e) => write!(f, "I/O error: {}", e),
            LaunchErrorKind::Collision(_) => write!(f, "route collisions detected"),
            LaunchErrorKind::FailedFairing => write!(f, "a launch fairing failed"),
            LaunchErrorKind::Unknown(ref e) => write!(f, "unknown error: {}", e)
        }
    }
}

impl fmt::Debug for LaunchError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.mark_handled();
        write!(f, "{:?}", self.kind())
    }
}

impl fmt::Display for LaunchError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.mark_handled();
        write!(f, "{}", self.kind())
    }
}

impl ::std::error::Error for LaunchError {
    #[inline]
    fn description(&self) -> &str {
        self.mark_handled();
        match *self.kind() {
            LaunchErrorKind::Io(_) => "an I/O error occured during launch",
            LaunchErrorKind::Collision(_) => "route collisions were detected",
            LaunchErrorKind::FailedFairing => "a launch fairing reported an error",
            LaunchErrorKind::Unknown(_) => "an unknown error occured during launch"
        }
    }
}

impl Drop for LaunchError {
    fn drop(&mut self) {
        if self.was_handled() {
            return
        }

        match *self.kind() {
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
            LaunchErrorKind::FailedFairing => {
                error!("Rocket failed to launch due to a failing fairing.");
                panic!("launch fairing failure");
            }
            LaunchErrorKind::Unknown(ref e) => {
                error!("Rocket failed to launch due to an unknown error.");
                panic!("{}", e);
            }
        }
    }
}
