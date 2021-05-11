use std::fmt;
use std::future::Future;

#[cfg(unix)]
use std::collections::HashSet;

use futures::future::{Either, pending};
use serde::{Deserialize, Serialize};

/// A Unix signal for triggering graceful shutdown.
///
/// Each variant corresponds to a Unix process signal which can be used to
/// trigger a graceful shutdown. See [`Shutdown`] for details.
///
/// ## (De)serialization
///
/// A `Sig` variant serializes and deserializes as a lowercase string equal to
/// the name of the variant: `"alrm"` for [`Sig::Alrm`], `"chld"` for
/// [`Sig::Chld`], and so on.
#[cfg(unix)]
#[cfg_attr(nightly, doc(cfg(unix)))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    Usr2
}

#[cfg(unix)]
#[cfg_attr(nightly, doc(cfg(unix)))]
impl fmt::Display for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
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
        };

        s.fmt(f)
    }
}

/// Graceful shutdown configuration.
///
/// # Summary
///
/// This structure configures when and how graceful shutdown occurs. The `ctrlc`
/// and `signals` properties control _when_ and the `grace` and `mercy`
/// properties control _how_.
///
/// When a shutdown is triggered by an externally or internally initiated
/// [`Shutdown::notify()`], Rocket allows application I/O to make progress for
/// at most `grace` seconds before initiating connection-level shutdown.
/// Connection shutdown forcibly terminates _application_ I/O, but connections
/// are allowed an additional `mercy` seconds to shutdown before being
/// forcefully terminated. This implies that a _cooperating_ and active remote
/// client maintaining an open connection can stall shutdown for at most `grace`
/// seconds, while an _uncooperative_ remote client can stall shutdown for at
/// most `grace + mercy` seconds.
///
/// # Triggers
///
/// _All_ graceful shutdowns are initiated via [`Shutdown::notify()`]. Rocket
/// can be configured to call [`Shutdown::notify()`] automatically on certain
/// conditions, specified via the `ctrlc` and `signals` properties of this
/// structure. More specifically, if `ctrlc` is `true` (the default), `ctrl-c`
/// (`SIGINT`) initiates a server shutdown, and on Unix, `signals` specifies a
/// list of IPC signals that trigger a shutdown (`["term"]` by default).
///
/// [`Shutdown::notify()`]: crate::Shutdown::notify()
///
/// # Grace Period
///
/// Once a shutdown is triggered, Rocket stops accepting new connections and
/// waits at most `grace` seconds before initiating connection shutdown.
/// Applications can `await` the [`Shutdown`](crate::Shutdown) future to detect
/// a shutdown and cancel any server-initiated I/O, such as from [infinite
/// responders](crate::response::stream#graceful-shutdown), to avoid abrupt I/O
/// cancellation.
///
/// # Mercy Period
///
/// After the grace period has elapsed, Rocket initiates connection shutdown,
/// allowing connection-level I/O termination such as TLS's `close_notify` to
/// proceed nominally. Rocket waits at most `mercy` seconds for connections to
/// shutdown before forcefully terminating all connections.
///
/// # Example
///
/// As with all Rocket configuration options, when using the default
/// [`Config::figment()`](crate::Config::figment()), `Shutdown` can be
/// configured via a `Rocket.toml` file. As always, defaults are provided
/// (documented below), and thus configuration only needs to provided to change
/// defaults.
///
/// ```rust
/// # use rocket::figment::{Figment, providers::{Format, Toml}};
/// use rocket::{Rocket, Config};
///
/// // If these are the contents of `Rocket.toml`...
/// # let toml = Toml::string(r#"
/// [default.shutdown]
/// ctrlc = false
/// signals = ["term", "hup"]
/// grace = 10
/// mercy = 5
/// # "#).nested();
///
/// // The config parses as follows:
/// # let config = Config::from(Figment::from(Config::debug_default()).merge(toml));
/// assert_eq!(config.shutdown.ctrlc, false);
/// assert_eq!(config.shutdown.grace, 10);
/// assert_eq!(config.shutdown.mercy, 5);
///
/// # #[cfg(unix)] {
/// use rocket::config::Sig;
///
/// assert_eq!(config.shutdown.signals.len(), 2);
/// assert!(config.shutdown.signals.contains(&Sig::Term));
/// assert!(config.shutdown.signals.contains(&Sig::Hup));
/// # }
/// ```
///
/// Or, as with all configuration options, programatically:
///
/// ```rust
/// # use rocket::figment::{Figment, providers::{Format, Toml}};
/// use rocket::{Rocket, Config};
/// use rocket::config::Shutdown;
///
/// #[cfg(unix)]
/// use rocket::config::Sig;
///
/// let config = Config {
///     shutdown: Shutdown {
///         ctrlc: false,
///         #[cfg(unix)]
///         signals: {
///             let mut set = std::collections::HashSet::new();
///             set.insert(Sig::Term);
///             set.insert(Sig::Hup);
///             set
///         },
///         grace: 10,
///         mercy: 5,
///     },
///     ..Config::default()
/// };
///
/// assert_eq!(config.shutdown.ctrlc, false);
/// assert_eq!(config.shutdown.grace, 10);
/// assert_eq!(config.shutdown.mercy, 5);
///
/// #[cfg(unix)] {
///     assert_eq!(config.shutdown.signals.len(), 2);
///     assert!(config.shutdown.signals.contains(&Sig::Term));
///     assert!(config.shutdown.signals.contains(&Sig::Hup));
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shutdown {
    /// Whether `ctrl-c` (`SIGINT`) initiates a server shutdown.
    ///
    /// **default: `true`**
    #[serde(deserialize_with = "figment::util::bool_from_str_or_int")]
    pub ctrlc: bool,
    /// On Unix, a set of signal which trigger a shutdown. On non-Unix, this
    /// option is unavailable and silently ignored.
    ///
    /// **default: { [`Sig::Term`] }**
    #[cfg(unix)]
    #[cfg_attr(nightly, doc(cfg(unix)))]
    pub signals: HashSet<Sig>,
    /// The grace period: number of seconds to continue to try to finish
    /// outstanding _server_ I/O for before forcibly terminating it.
    ///
    /// **default: `2`**
    pub grace: u32,
    /// The mercy period: number of seconds to continue to try to finish
    /// outstanding _connection_ I/O for before forcibly terminating it.
    ///
    /// **default: `3`**
    pub mercy: u32,
}

impl fmt::Display for Shutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ctrlc = {}, ", self.ctrlc)?;

        #[cfg(unix)] {
            write!(f, "signals = [")?;
            for (i, sig) in self.signals.iter().enumerate() {
                if i != 0 { write!(f, ", ")?; }
                write!(f, "{}", sig)?;
            }
            write!(f, "], ")?;
        }

        write!(f, "grace = {}s, mercy = {}s", self.grace, self.mercy)?;
        Ok(())
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Shutdown {
            ctrlc: true,
            #[cfg(unix)]
            signals: { let mut set = HashSet::new(); set.insert(Sig::Term); set },
            grace: 2,
            mercy: 3,
        }
    }
}

impl Shutdown {
    #[cfg(unix)]
    pub(crate) fn collective_signal(&self) -> impl Future<Output = ()> {
        use futures::future::{FutureExt, select_all};
        use tokio::signal::unix::{signal, SignalKind};

        if !self.ctrlc && self.signals.is_empty() {
            return Either::Right(pending());
        }

        let mut signals = self.signals.clone();
        if self.ctrlc {
            signals.insert(Sig::Int);
        }

        let mut sigfuts = vec![];
        for sig in signals {
            let sigkind = match sig {
                Sig::Alrm => SignalKind::alarm(),
                Sig::Chld => SignalKind::child(),
                Sig::Hup => SignalKind::hangup(),
                Sig::Int => SignalKind::interrupt(),
                Sig::Io => SignalKind::io(),
                Sig::Pipe => SignalKind::pipe(),
                Sig::Quit => SignalKind::quit(),
                Sig::Term => SignalKind::terminate(),
                Sig::Usr1 => SignalKind::user_defined1(),
                Sig::Usr2 => SignalKind::user_defined2()
            };

            let sigfut = match signal(sigkind) {
                Ok(mut signal) => Box::pin(async move {
                    signal.recv().await;
                    warn!("Received {} signal. Requesting shutdown.", sig);
                }),
                Err(e) => {
                    warn!("Failed to enable `{}` shutdown signal.", sig);
                    info_!("Error: {}", e);
                    continue
                }
            };

            sigfuts.push(sigfut);
        }

        Either::Left(select_all(sigfuts).map(|_| ()))
    }

    #[cfg(not(unix))]
    pub(crate) fn collective_signal(&self) -> impl Future<Output = ()> {
        use futures::future::FutureExt;

        match self.ctrlc {
            true => Either::Left(tokio::signal::ctrl_c().map(|result| {
                if let Err(e) = result {
                    warn!("Failed to enable `ctrl-c` shutdown signal.");
                    info_!("Error: {}", e);
                }
            })),
            false => Either::Right(pending()),
        }
    }
}
