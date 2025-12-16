use std::fmt;

use serde::{Deserialize, Serialize};

/// A Unix signal for triggering graceful shutdown.
///
/// Each variant corresponds to a Unix process signal which can be used to
/// trigger a graceful shutdown. See [`Shutdown`](crate::Shutdown) for details.
///
/// ## (De)serialization
///
/// A `Sig` variant serializes and deserializes as a lowercase string equal to
/// the name of the variant: `"alrm"` for [`Sig::Alrm`], `"chld"` for
/// [`Sig::Chld`], and so on.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(nightly, doc(cfg(unix)))]
pub enum Sig {
    /// The `SIGALRM` Unix signal.
    Alrm,
    /// The `SIGCHLD` Unix signal.
    Chld,
    /// The `SIGHUP` Unix signal.
    Hup,
    /// The `SIGINT` Unix signal.
    Int,
    /// The `SIGIO` Unix signal.
    Io,
    /// The `SIGPIPE` Unix signal.
    Pipe,
    /// The `SIGQUIT` Unix signal.
    Quit,
    /// The `SIGTERM` Unix signal.
    Term,
    /// The `SIGUSR1` Unix signal.
    Usr1,
    /// The `SIGUSR2` Unix signal.
    Usr2,
}

impl Sig {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sig::Alrm => "SIGALRM",
            Sig::Chld => "SIGCHLD",
            Sig::Hup => "SIGHUP",
            Sig::Int => "SIGINT",
            Sig::Io => "SIGIO",
            Sig::Pipe => "SIGPIPE",
            Sig::Quit => "SIGQUIT",
            Sig::Term => "SIGTERM",
            Sig::Usr1 => "SIGUSR1",
            Sig::Usr2 => "SIGUSR2",
        }
    }
}

impl fmt::Display for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
