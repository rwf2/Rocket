use std::{io, fmt};
use std::str::FromStr;
use std::path::PathBuf;
use std::net::{IpAddr, ToSocketAddrs};

/// The configured address to serve a Rocket application on.
///
/// Addresses can be specified as either IP addresses or hostnames. On Unix
/// platforms, addresses can also be paths to Unix domain sockets specified as
/// `unix:path/to/socket`.
///
/// When a hostname is specified, it is resolved to the _first available_ IP
/// address for the given hostname. Rocket _does not_ bind to all addresses
/// available for a given hostname. For example, if both `127.0.0.1` and `::1`
/// are specified for `localhost`, the first address available of the two will
/// be used, and the other address will be ignored by Rocket.
///
/// When a Unix domain socket is specified, a lock file named
/// `path/to/socket.lock` is created and locked. If locking fails, application
/// launch is aborted. Additionally, if `path/to/socket` does not exist, it is
/// created. Both the lock file and the socket file are unconditionally deleted
/// upon server termination.
#[derive(Debug, Clone, PartialEq)]
pub enum Address {
    /// The hostname to serve over TCP.
    Hostname(String),
    /// The IP address to serve over TCP.
    Ip(IpAddr),
    /// The path to the unix domain socket.
    Unix(PathBuf),
}

impl Address {
    crate const UNIX_PREFIX: &'static str = "unix:";

    crate fn is_unix(&self) -> bool {
        match self {
            Address::Unix(..) => true,
            _ => false
        }
    }
}

impl FromStr for Address {
    type Err = io::Error;

    fn from_str(string: &str) -> io::Result<Self> {
        #[cfg(unix)]
        {
            if string.starts_with(Address::UNIX_PREFIX) {
                let address = &string[Address::UNIX_PREFIX.len()..];
                return Ok(Address::Unix(address.into()));
            }
        }

        // Use `to_socket_addr` to check for address resolution, _not_ for parsing.
        if (string, 0).to_socket_addrs()?.next().is_some() {
            if let Ok(ip) = IpAddr::from_str(string) {
                return Ok(Address::Ip(ip));
            } else {
                return Ok(Address::Hostname(string.into()));
            }
        }

        Err(io::Error::new(io::ErrorKind::Other, "failed to resolve TCP address"))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Hostname(name) => name.fmt(f),
            Address::Ip(addr) => addr.fmt(f),
            Address::Unix(path) => write!(f, "{}{}", Address::UNIX_PREFIX, path.display()),
        }
    }
}
