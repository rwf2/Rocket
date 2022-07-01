mod parser;
mod error;
pub(crate) mod tables;

#[cfg(test)] mod tests;

use crate::uri::{Uri, Origin, Absolute, Authority, Reference, Asterisk};

use self::parser::*;

pub use self::error::Error;

type RawInput<'a> = pear::input::Pear<pear::input::Cursor<&'a [u8]>>;

#[inline]
pub fn from_str(s: &str) -> Result<Uri<'_>, Error<'_>> {
    Ok(parse!(uri: RawInput::new(s.as_bytes()))?)
}

#[inline]
pub fn origin_from_str(s: &str) -> Result<Origin<'_>, Error<'_>> {
    Ok(parse!(origin: RawInput::new(s.as_bytes()))?)
}

#[inline]
pub fn authority_from_str(s: &str) -> Result<Authority<'_>, Error<'_>> {
    Ok(parse!(authority: RawInput::new(s.as_bytes()))?)
}

#[inline]
pub fn authority_from_bytes(s: &[u8]) -> Result<Authority<'_>, Error<'_>> {
    Ok(parse!(authority: RawInput::new(s))?)
}

#[inline]
pub fn scheme_from_str(s: &str) -> Result<&str, Error<'_>> {
    let _validated = parse!(scheme: RawInput::new(s.as_bytes()))?;
    Ok(s)
}

#[inline]
pub fn absolute_from_str(s: &str) -> Result<Absolute<'_>, Error<'_>> {
    Ok(parse!(absolute: RawInput::new(s.as_bytes()))?)
}

#[inline]
pub fn asterisk_from_str(s: &str) -> Result<Asterisk, Error<'_>> {
    Ok(parse!(asterisk: RawInput::new(s.as_bytes()))?)
}

#[inline]
pub fn reference_from_str(s: &str) -> Result<Reference<'_>, Error<'_>> {
    Ok(parse!(reference: RawInput::new(s.as_bytes()))?)
}
