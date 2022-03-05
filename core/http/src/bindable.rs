//! Exports `BindableAddr` to offer generic operations over addresses that can be bound to.

use serde_::de::{Deserializer, self};
use serde_::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, self};
use std::net::{SocketAddr, IpAddr, AddrParseError};
use std::path::PathBuf;
use std::str::FromStr;

/// All types of addresses that can be used for the server
#[derive(PartialEq, Clone, Debug, Serialize)]
#[serde(crate = "serde_", into = "SerdeHelper")]
pub enum BindableAddr {
    /// Listen on an address and port with the TCP protocol
    Tcp(SocketAddr),
    /// Listen on an address and port with the UDP protocol
    Udp(SocketAddr),
    /// Listen on a Unix socket
    Unix(PathBuf),
}

impl BindableAddr {
    /// The name of the protocol that would be used for this address in its string representation
    pub fn protocol_name(&self) -> &'static str {
        match self {
            Self::Tcp(_) => "tcp",
            Self::Udp(_) => "udp",
            Self::Unix(_) => "unix",
        }
    }
    /// The IP address, if the inner address type has one
    pub fn ip(&self) -> Option<IpAddr> {
        match self {
            Self::Tcp(addr) | Self::Udp(addr) => Some(addr.ip()),
            _ => None,
        }
    }
    /// The port, if the inner address type has one
    pub fn port(&self) -> Option<u16> {
        match self {
            Self::Tcp(addr) | Self::Udp(addr) => Some(addr.port()),
            _ => None,
        }
    }
}

impl Display for BindableAddr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp(addr) | Self::Udp(addr) => write!(formatter, "{}://{}", self.protocol_name(), addr),
            Self::Unix(path) => write!(formatter, "{}://{}", self.protocol_name(), path.display()),
        }
    }
}

/// The possible errors when parsing a `BindableAddr` from a string
#[derive(Debug)]
pub enum FromStrError {
    /// The protocol (e.g., "tcp" in "tcp://127.0.0.1:8080") was unrecognized
    UnknownProtocol(String),
    /// A raw IP address was received that requires a port to be used as a `BindableAddr`
    RequiresPort(IpAddr),
    /// For protocols that take an IP address, that IP address could not be parsed
    Ip(AddrParseError),
}

impl Display for FromStrError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProtocol(unknown) => write!(formatter, "unknown protocol {:?}", unknown),
            Self::RequiresPort(addr) => write!(formatter, "raw IP address {:?} requires a port", addr),
            Self::Ip(err) => write!(formatter, "invalid IP: {}", err),
        }
    }
}
impl From<AddrParseError> for FromStrError {
    fn from(err: AddrParseError) -> Self {
        Self::Ip(err)
    }
}
impl std::error::Error for FromStrError {}

impl FromStr for BindableAddr {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((protocol_name, data)) = s.split_once("://") {
            Ok(match protocol_name {
                "tcp" => Self::Tcp(SocketAddr::from_str(data)?),
                "udp" => Self::Udp(SocketAddr::from_str(data)?),
                "unix" => Self::Unix(PathBuf::from(data)),
                unknown => return Err(Self::Err::UnknownProtocol(unknown.to_string())),
            })
        } else {
            Err(Self::Err::RequiresPort(IpAddr::from_str(s)?))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "serde_")]
struct SerdeHelper {
    address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
}
impl From<BindableAddr> for SerdeHelper {
    fn from(addr: BindableAddr) -> Self {
        Self { address: addr.to_string(), port: None }
    }
}

impl<'de> Deserialize<'de> for BindableAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
        D::Error: de::Error,
    {
        let SerdeHelper { address: addr, port } = SerdeHelper::deserialize(deserializer)?;
        match Self::from_str(&addr) {
            Err(FromStrError::RequiresPort(addr)) => {
                if let Some(port) = port {
                    let converted = Self::Tcp(SocketAddr::new(addr, port));
                    log::warn!("Raw addresses are deprecated. Please use a protocol address in the `address` config field and remove `port`. Here is the value to use for `address`: {:?}", converted.to_string());
                    Ok(converted)
                } else {
                    Err(de::Error::custom("No port provided with raw address"))
                }
            },
            other => {
                if port.is_some() {
                    log::warn!("`port` config field is ignored when `address` is a protocol address. Please remove it.");
                }
                other.map_err(de::Error::custom)
            },
        }
    }
}
