use std::ops::BitOr;

/// Information about a [`Fairing`](crate::fairing::Fairing).
///
/// The `name` field is an arbitrary name for a fairing. The `kind` field is a
/// is an `or`d set of [`Kind`] structures. Rocket uses the values set in `Kind`
/// to determine which callbacks from a given `Fairing` implementation to
/// actually call.
///
/// # Example
///
/// A simple `Info` structure that can be used for a `Fairing` that implements
/// all callbacks:
///
/// ```
/// use rocket::fairing::{Info, Kind};
///
/// # let _unused_info =
/// Info {
///     name: "Example Fairing",
///     kind: Kind::Ignite | Kind::Liftoff | Kind::Request | Kind::Response | Kind::Shutdown
/// }
/// # ;
/// ```
#[derive(Debug, Copy, Clone)]
pub struct Info {
    /// The name of the fairing.
    pub name: &'static str,
    /// A set representing the callbacks the fairing wishes to receive.
    pub kind: Kind,
}

/// A bitset representing the kinds of callbacks a
/// [`Fairing`](crate::fairing::Fairing) wishes to receive.
///
/// A fairing can request any combination of any of the following kinds of
/// callbacks:
///
///   * Ignite
///   * Liftoff
///   * Request
///   * Response
///   * Shutdown
///
/// Two `Kind` structures can be `or`d together to represent a combination. For
/// instance, to represent a fairing that is both an ignite and request fairing,
/// use `Kind::Ignite | Kind::Request`. Similarly, to represent a fairing that
/// is only an ignite fairing, use `Kind::Ignite`.
///
/// Additionally, a fairing can request to be treated as a
/// [singleton](crate::fairing::Fairing#singletons) by specifying the
/// `Singleton` kind.
#[derive(Debug, Clone, Copy)]
pub struct Kind(usize);

#[allow(non_upper_case_globals)]
impl Kind {
    /// `Kind` flag representing a request for a 'ignite' callback.
    pub const Ignite: Kind = Kind(1 << 0);

    /// `Kind` flag representing a request for a 'liftoff' callback.
    pub const Liftoff: Kind = Kind(1 << 1);

    /// `Kind` flag representing a request for a 'request' callback.
    pub const Request: Kind = Kind(1 << 2);

    /// `Kind` flag representing a request for a 'response' callback.
    pub const Response: Kind = Kind(1 << 3);

    /// `Kind` flag representing a request for a 'shutdown' callback.
    pub const Shutdown: Kind = Kind(1 << 4);

    /// `Kind` flag representing a
    /// [singleton](crate::fairing::Fairing#singletons) fairing.
    pub const Singleton: Kind = Kind(1 << 5);

    /// Returns `true` if `self` is a superset of `other`. In other words,
    /// returns `true` if all of the kinds in `other` are also in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::Kind;
    ///
    /// let ignite_and_req = Kind::Ignite | Kind::Request;
    /// assert!(ignite_and_req.is(Kind::Ignite | Kind::Request));
    ///
    /// assert!(ignite_and_req.is(Kind::Ignite));
    /// assert!(ignite_and_req.is(Kind::Request));
    ///
    /// assert!(!ignite_and_req.is(Kind::Liftoff));
    /// assert!(!ignite_and_req.is(Kind::Response));
    /// assert!(!ignite_and_req.is(Kind::Ignite | Kind::Response));
    /// assert!(!ignite_and_req.is(Kind::Ignite | Kind::Request | Kind::Response));
    /// ```
    #[inline]
    pub fn is(self, other: Kind) -> bool {
        (other.0 & self.0) == other.0
    }

    /// Returns `true` if `self` is exactly `other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fairing::Kind;
    ///
    /// let ignite_and_req = Kind::Ignite | Kind::Request;
    /// assert!(ignite_and_req.is_exactly(Kind::Ignite | Kind::Request));
    ///
    /// assert!(!ignite_and_req.is_exactly(Kind::Ignite));
    /// assert!(!ignite_and_req.is_exactly(Kind::Request));
    /// assert!(!ignite_and_req.is_exactly(Kind::Response));
    /// assert!(!ignite_and_req.is_exactly(Kind::Ignite | Kind::Response));
    /// ```
    #[inline]
    pub fn is_exactly(self, other: Kind) -> bool {
        self.0 == other.0
    }
}

impl BitOr for Kind {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Kind(self.0 | rhs.0)
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut is_first = true;
        let mut write = |string, kind| {
            if self.is(kind) {
                if !is_first { f.write_str(", ")?; }
                f.write_str(string)?;
                is_first = false;
            }

            Ok(())
        };

        write("ignite", Kind::Ignite)?;
        write("liftoff", Kind::Liftoff)?;
        write("request", Kind::Request)?;
        write("response", Kind::Response)?;
        write("shutdown", Kind::Shutdown)?;
        write("singleton", Kind::Singleton)
    }
}
