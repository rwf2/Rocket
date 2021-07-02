use pear::parsers::*;
use pear::combinators::*;
use pear::input::{self, Pear, Extent, Rewind, Input};
use pear::macros::{parser, switch, parse_error, parse_try};

use crate::uri::{Uri, Origin, Authority, Absolute, Reference, Asterisk};
use crate::parse::uri::tables::*;
use crate::parse::uri::RawInput;

type Result<'a, T> = pear::input::Result<T, RawInput<'a>>;

// SAFETY: Every `unsafe` here comes from bytes -> &str conversions. Since all
// bytes are checked against tables in `tables.rs`, all of which allow only
// ASCII characters, these are all safe.

// TODO: We should cap the source we pass into `raw` to the bytes we've actually
// checked. Otherwise, we could have good bytes followed by unchecked bad bytes
// since eof() may not called. However, note that we only actually expose these
// parsers via `parse!()`, which _does_ call `eof()`, so we're externally okay.

#[parser(rewind)]
pub fn complete<I, P, O>(input: &mut Pear<I>, p: P) -> input::Result<O, I>
    where I: Input + Rewind, P: FnOnce(&mut Pear<I>) -> input::Result<O, I>
{
    (p()?, eof()?).0
}

/// TODO: Have a way to ask for for preference in ambiguity resolution.
///   * An ordered [Preference] is probably best.
///   * Need to filter/uniqueify. See `uri-pref`.
/// Once we have this, we should probably set the default so that `authority` is
/// preferred over `absolute`, otherwise something like `foo:3122` is absolute.
#[parser]
pub fn uri<'a>(input: &mut RawInput<'a>) -> Result<'a, Uri<'a>> {
    // To resolve all ambiguities with preference, we might need to look at the
    // complete string twice: origin/ref, asterisk/ref, authority/absolute.
    switch! {
        asterisk@complete(asterisk) => Uri::Asterisk(asterisk),
        origin@complete(origin) => Uri::Origin(origin),
        authority@complete(authority) => Uri::Authority(authority),
        absolute@complete(absolute) => Uri::Absolute(absolute),
        _ => Uri::Reference(reference()?)
    }
}

#[parser]
pub fn asterisk<'a>(input: &mut RawInput<'a>) -> Result<'a, Asterisk> {
    eat(b'*')?;
    Asterisk
}

#[parser]
pub fn origin<'a>(input: &mut RawInput<'a>) -> Result<'a, Origin<'a>> {
    let (_, path, query) = (peek(b'/')?, path()?, query()?);
    unsafe { Origin::raw(input.start.into(), path, query) }
}

#[parser]
pub fn authority<'a>(input: &mut RawInput<'a>) -> Result<'a, Authority<'a>> {
    let prefix = take_while(is_reg_name_char)?;
    let (user_info, host, port) = switch! {
        peek(b'[') if prefix.is_empty() => (None, host()?, port()?),
        eat(b':') => {
            let suffix = take_while(is_reg_name_char)?;
            switch! {
                peek(b':') => {
                    let end = (take_while(is_user_info_char)?, eat(b'@')?).0;
                    (input.span(prefix, end), host()?, port()?)
                },
                eat(b'@') => (input.span(prefix, suffix), host()?, port()?),
                // FIXME: Rewind to just after prefix to get the right context
                // to be able to call `port()`. Then remove `maybe_port()`.
                _ => (None, prefix, maybe_port(&suffix)?)
            }
        },
        eat(b'@') => (Some(prefix), host()?, port()?),
        _ => (None, prefix, None),
    };

    unsafe { Authority::raw(input.start.into(), user_info, host, port) }
}

#[parser]
pub fn scheme<'a>(input: &mut RawInput<'a>) -> Result<'a, Extent<&'a [u8]>> {
    let scheme = take_some_while(is_scheme_char)?;
    if !scheme.get(0).map_or(false, |b| b.is_ascii_alphabetic()) {
        parse_error!("invalid scheme")?;
    }

    scheme
}

#[parser]
pub fn absolute<'a>(input: &mut RawInput<'a>) -> Result<'a, Absolute<'a>> {
    let scheme = scheme()?;
    let (_, (authority, path), query) = (eat(b':')?, hier_part()?, query()?);
    unsafe { Absolute::raw(input.start.into(), scheme, authority, path, query) }
}

#[parser]
pub fn reference<'a>(
    input: &mut RawInput<'a>,
) -> Result<'a, Reference<'a>> {
    let prefix = take_while(is_scheme_char)?;
    let (scheme, authority, path) = switch! {
        peek(b':') if prefix.is_empty() => parse_error!("missing scheme")?,
        eat(b':') => {
            if !prefix.get(0).map_or(false, |b| b.is_ascii_alphabetic()) {
                parse_error!("invalid scheme")?;
            }

            let (authority, path) = hier_part()?;
            (Some(prefix), authority, path)
        },
        peek_slice(b"//") if prefix.is_empty() => {
            let (authority, path) = hier_part()?;
            (None, authority, path)
        },
        _ => {
            let path = path()?;
            let full_path = input.span(prefix, path).unwrap_or(none()?);
            (None, None, full_path)
        },
    };

    let (source, query, fragment) = (input.start.into(), query()?, fragment()?);
    unsafe { Reference::raw(source, scheme, authority, path, query, fragment) }
}

#[parser]
pub fn hier_part<'a>(
    input: &mut RawInput<'a>
) -> Result<'a, (Option<Authority<'a>>, Extent<&'a [u8]>)> {
    switch! {
        eat_slice(b"//") => {
            let authority = authority()?;
            let path = parse_try!(peek(b'/') => path()? => || none()?);
            (Some(authority), path)
        },
        _ => (None, path()?)
    }
}

#[parser]
fn host<'a>(
    input: &mut RawInput<'a>,
) -> Result<'a, Extent<&'a [u8]>> {
    switch! {
        peek(b'[') => enclosed(b'[', is_pchar, b']')?,
        _ => take_while(is_reg_name_char)?
    }
}

#[parser]
fn port<'a>(
    input: &mut RawInput<'a>,
) -> Result<'a, Option<u16>> {
    if !succeeds(input, |i| eat(i, b':')) {
        return Ok(None);
    }

    let bytes = take_n_while(5, |c| c.is_ascii_digit())?;
    maybe_port(&bytes)?
}

// FIXME: The context here is wrong since it's empty. We should reset to
// current - bytes.len(). Or something like that.
#[parser]
fn maybe_port<'a>(input: &mut RawInput<'a>, bytes: &[u8]) -> Result<'a, Option<u16>> {
    if bytes.len() > 5 {
        parse_error!("port len is out of range")?;
    } else if !bytes.iter().all(|b| b.is_ascii_digit()) {
        parse_error!("invalid port bytes")?;
    }

    let mut port_num: u32 = 0;
    for (b, i) in bytes.iter().rev().zip(&[1, 10, 100, 1000, 10000]) {
        port_num += (b - b'0') as u32 * i;
    }

    if port_num > u16::max_value() as u32 {
        parse_error!("port out of range: {}", port_num)?;
    }

    Some(port_num as u16)
}

#[parser]
fn path<'a>(input: &mut RawInput<'a>) -> Result<'a, Extent<&'a [u8]>> {
    take_while(is_pchar)?
}

#[parser]
fn query<'a>(input: &mut RawInput<'a>) -> Result<'a, Option<Extent<&'a [u8]>>> {
    parse_try!(eat(b'?') => take_while(is_qchar)?)
}

#[parser]
fn fragment<'a>(input: &mut RawInput<'a>) -> Result<'a, Option<Extent<&'a [u8]>>> {
    parse_try!(eat(b'#') => take_while(is_qchar)?)
}
