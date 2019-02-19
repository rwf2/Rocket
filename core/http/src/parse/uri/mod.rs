mod error;
mod parser;
mod tables;

#[cfg(test)]
mod tests;

use self::parser::{absolute_only, authority_only, origin, rocket_route_origin, uri};
use parse::indexed::IndexedInput;
use uri::{Absolute, Authority, Origin, Uri};

pub use self::error::Error;
crate use self::tables::is_pchar;

type RawInput<'a> = IndexedInput<'a, [u8]>;

#[inline]
pub fn from_str(string: &str) -> Result<Uri, Error> {
    parse!(uri: &mut RawInput::from(string.as_bytes())).map_err(|e| Error::from(string, e))
}

#[inline]
pub fn origin_from_str(string: &str) -> Result<Origin, Error> {
    parse!(origin: &mut RawInput::from(string.as_bytes())).map_err(|e| Error::from(string, e))
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
