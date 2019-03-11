mod parser;
mod error;
mod tables;

#[cfg(test)] mod tests;

use uri::{Uri, Origin, Absolute, Authority};
use parse::indexed::IndexedInput;
use self::parser::{uri, origin, authority_only, absolute_only, rocket_route_origin};

crate use self::tables::is_pchar;
pub use self::error::Error;

type RawInput<'a> = IndexedInput<'a, [u8]>;

#[inline]
pub fn from_str(string: &str) -> Result<Uri, Error> {
    parse!(uri: &mut RawInput::from(string.as_bytes()))
        .map_err(|e| Error::from(string, e))
}

#[inline]
pub fn origin_from_str(string: &str) -> Result<Origin, Error> {
    parse!(origin: &mut RawInput::from(string.as_bytes()))
        .map_err(|e| Error::from(string, e))
}

#[inline]
pub fn route_origin_from_str(string: &str) -> Result<Origin, Error> {
    parse!(rocket_route_origin: &mut RawInput::from(string.as_bytes()))
        .map_err(|e| Error::from(string, e))
}

#[inline]
pub fn authority_from_str(string: &str) -> Result<Authority, Error> {
    parse!(authority_only: &mut RawInput::from(string.as_bytes()))
        .map_err(|e| Error::from(string, e))
}

#[inline]
pub fn absolute_from_str(string: &str) -> Result<Absolute, Error> {
    parse!(absolute_only: &mut RawInput::from(string.as_bytes()))
        .map_err(|e| Error::from(string, e))
}
