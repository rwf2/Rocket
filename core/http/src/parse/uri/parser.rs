use pear::parsers::*;
use pear::{parser, switch};

use crate::uri::{Uri, Origin, Authority, Absolute, Host};
use crate::parse::uri::tables::{is_reg_name_char, is_pchar, is_pchar_or_rchar};
use crate::parse::uri::RawInput;
use crate::parse::IndexedBytes;

type Result<'a, T> = pear::Result<T, RawInput<'a>>;

#[parser]
crate fn uri<'a>(input: &mut RawInput<'a>) -> Result<'a, Uri<'a>> {
    match input.len() {
        0 => return Err(pear_error!("empty URI")),
        1 => switch! {
            eat(b'*') => Uri::Asterisk,
            eat(b'/') => Uri::Origin(Origin::new::<_, &str>("/", None)),
            _ => unsafe {
                // the `is_reg_name_char` guarantees ASCII
                let host = Host::Raw(take_n_if(1, is_reg_name_char)?);
                Uri::Authority(Authority::raw(input.cow_source(), None, host, None))
            }
        },
        _ => switch! {
            peek(b'/') => Uri::Origin(origin()?),
            _ => absolute_or_authority()?
        }
    }
}

#[parser]
crate fn origin<'a>(input: &mut RawInput<'a>) -> Result<'a, Origin<'a>> {
    (peek(b'/')?, path_and_query(is_pchar)?).1
}

#[parser]
crate fn rocket_route_origin<'a>(input: &mut RawInput<'a>) -> Result<'a, Origin<'a>> {
    (peek(b'/')?, path_and_query(is_pchar_or_rchar)?).1
}

#[parser]
fn path_and_query<'a, F>(input: &mut RawInput<'a>, is_good_char: F) -> Result<'a, Origin<'a>>
    where F: Fn(u8) -> bool + Copy
{
    let path = take_while(is_good_char)?;

    // FIXME(rustc): We should be able to use `pear_try`, but rustc...is broken.
    let query = switch! {
        eat(b'?') => Some(take_while(|c| is_good_char(c) || c == b'?')?),
        _ => None
    };

    if path.is_empty() && query.is_none() {
        Err(pear_error!("expected path or query, found neither"))
    } else {
        // We know the string is ASCII because of the `is_good_char` checks above.
        Ok(unsafe { Origin::raw(input.cow_source(), path, query) })
    }
}

#[parser]
fn port_from<'a>(input: &mut RawInput<'a>, bytes: &IndexedBytes<'a>) -> Result<'a, u16> {
    let mut port_num: u32 = 0;
    let source = Some(input.cow_source());
    let string = bytes.from_cow_source(&source);
    for (&b, i) in string.iter().rev().zip(&[1, 10, 100, 1000, 10000]) {
        if b < b'0' || b > b'9' {
            return Err(pear_error!("port byte is out of range"));
        }

        port_num += (b - b'0') as u32 * i;
    }

    if port_num > u16::max_value() as u32 {
        return Err(pear_error!("port value out of range: {}", port_num));
    }

    Ok(port_num as u16)
}

#[parser]
fn port<'a>(input: &mut RawInput<'a>) -> Result<'a, u16> {
    let port_str = take_n_while(5, |c| c >= b'0' && c <= b'9')?;
    port_from(&port_str)?
}

#[parser]
fn authority<'a>(
    input: &mut RawInput<'a>,
    user_info: Option<IndexedBytes<'a>>
) -> Result<'a, Authority<'a>> {
    let host = switch! {
        peek(b'[') => Host::Bracketed(delimited(b'[', is_pchar, b']')?),
        _ => Host::Raw(take_while(is_reg_name_char)?)
    };

    // The `is_pchar`,`is_reg_name_char`, and `port()` functions ensure ASCII.
    let port = pear_try!(eat(b':') => port()?);
    unsafe { Authority::raw(input.cow_source(), user_info, host, port) }
}

// Callers must ensure that `scheme` is actually ASCII.
#[parser]
fn absolute<'a>(
    input: &mut RawInput<'a>,
    scheme: IndexedBytes<'a>
) -> Result<'a, Absolute<'a>> {
    let (authority, path_and_query) = switch! {
        eat_slice(b"://") => {
            let left = take_while(|c| is_reg_name_char(c) || c == b':')?;
            let authority = switch! {
                eat(b'@') => authority(Some(left))?,
                _ => {
                    input.backtrack(left.len())?;
                    authority(None)?
                }
            };

            let path_and_query = pear_try!(path_and_query(is_pchar));
            (Some(authority), path_and_query)
        },
        eat(b':') => (None, Some(path_and_query(is_pchar)?)),
        _ => return Err(pear_error!("expected ':' but none was found"))
    };

    // `authority` and `path_and_query` parsers ensure ASCII.
    unsafe { Absolute::raw(input.cow_source(), scheme, authority, path_and_query) }
}

#[parser]
crate fn authority_only<'a>(input: &mut RawInput<'a>) -> Result<'a, Authority<'a>> {
    if let Uri::Authority(authority) = absolute_or_authority()? {
        Ok(authority)
    } else {
        Err(pear_error!("expected authority URI but found absolute URI"))
    }
}

#[parser]
crate fn absolute_only<'a>(input: &mut RawInput<'a>) -> Result<'a, Absolute<'a>> {
    if let Uri::Absolute(absolute) = absolute_or_authority()? {
        Ok(absolute)
    } else {
        Err(pear_error!("expected absolute URI but found authority URI"))
    }
}

#[parser]
fn absolute_or_authority<'a>(
    input: &mut RawInput<'a>,
) -> Result<'a, Uri<'a>> {
    let left = take_while(is_reg_name_char)?;
    switch! {
        peek_slice(b":/") => Uri::Absolute(absolute(left)?),
        eat(b'@') => Uri::Authority(authority(Some(left))?),
        colon@take_n_if(1, |b| b == b':') => {
            // could be authority or an IP with ':' in it
            let rest = take_while(|c| is_reg_name_char(c) || c == b':')?;
            switch! {
                eat(b'@') => Uri::Authority(authority(Some(left + colon + rest))?),
                peek(b'/') => {
                    input.backtrack(rest.len() + 1)?;
                    Uri::Absolute(absolute(left)?)
                },
                _ => unsafe {
                    // Here we hit an ambiguity: `rest` could be a port in
                    // host:port or a host in scheme:host. Both are correct
                    // parses. To settle the ambiguity, we assume that if it
                    // looks like a port, it's a port. Otherwise a host. Unless
                    // we have a query, in which case it's definitely a host.
                    let query = pear_try!(eat(b'?') => take_while(is_pchar)?);
                    if query.is_some() || rest.is_empty() || rest.len() > 5 {
                        Uri::raw_absolute(input.cow_source(), left, rest, query)
                    } else if let Ok(port) = port_from(input, &rest) {
                        let host = Host::Raw(left);
                        let source = input.cow_source();
                        let port = Some(port);
                        Uri::Authority(Authority::raw(source, None, host, port))
                    } else {
                        Uri::raw_absolute(input.cow_source(), left, rest, query)
                    }
                }
            }
        },
        _ => {
            input.backtrack(left.len())?;
            Uri::Authority(authority(None)?)
        }
    }
}
