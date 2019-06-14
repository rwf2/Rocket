use std::borrow::Cow;
use std::marker::PhantomData;

use unicode_xid::UnicodeXID;

use crate::ext::IntoOwned;
use crate::uri::{Origin, UriPart, Path, Query};
use crate::uri::encoding::unsafe_percent_encode;

use self::Error::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Static,
    Single,
    Multi,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    Path,
    Query,
    Data,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RouteSegment<'a, P: UriPart> {
    pub string: Cow<'a, str>,
    pub kind: Kind,
    pub name: Cow<'a, str>,
    pub index: Option<usize>,
    _part: PhantomData<P>,
}

impl<P: UriPart + 'static> IntoOwned for RouteSegment<'_, P> {
    type Owned = RouteSegment<'static, P>;

    #[inline]
    fn into_owned(self) -> Self::Owned {
        RouteSegment {
            string: IntoOwned::into_owned(self.string),
            kind: self.kind,
            name: IntoOwned::into_owned(self.name),
            index: self.index,
            _part: PhantomData
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Error<'a> {
    Empty,
    Ident(&'a str),
    Ignored,
    MissingClose,
    Malformed,
    Uri,
    Trailing(&'a str)
}

pub type SResult<'a, P> = Result<RouteSegment<'a, P>, (&'a str, Error<'a>)>;

#[inline]
fn is_ident_start(c: char) -> bool {
    ('a' <= c && c <= 'z')
        || ('A' <= c && c <= 'Z')
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_start(c))
}

#[inline]
fn is_ident_continue(c: char) -> bool {
    ('a' <= c && c <= 'z')
        || ('A' <= c && c <= 'Z')
        || c == '_'
        || ('0' <= c && c <= '9')
        || (c > '\x7f' && UnicodeXID::is_xid_continue(c))
}

fn is_valid_ident(string: &str) -> bool {
    let mut chars = string.chars();
    match chars.next() {
        Some(c) => is_ident_start(c) && chars.all(is_ident_continue),
        None => false
    }
}

impl<'a, P: UriPart> RouteSegment<'a, P> {
    pub fn parse_one(segment: &'a str) -> Result<Self, Error<'_>> {
        let (string, index) = (segment.into(), None);

        // Check if this is a dynamic param. If so, check its well-formedness.
        if segment.starts_with('<') && segment.ends_with('>') {
            let mut kind = Kind::Single;
            let mut name = &segment[1..(segment.len() - 1)];
            if name.ends_with("..") {
                kind = Kind::Multi;
                name = &name[..(name.len() - 2)];
            }

            if name.is_empty() {
                return Err(Empty);
            } else if !is_valid_ident(name) {
                return Err(Ident(name));
            } else if name == "_" {
                return Err(Ignored);
            }

            let name = name.into();
            return Ok(RouteSegment { string, name, kind, index, _part: PhantomData });
        } else if segment.is_empty() {
            return Err(Empty);
        } else if segment.starts_with('<') && segment.len() > 1
                && !segment[1..].contains('<') && !segment[1..].contains('>') {
            return Err(MissingClose);
        } else if segment.contains('>') || segment.contains('<') {
            return Err(Malformed);
        } else if unsafe_percent_encode::<P>(segment) != segment {
            return Err(Uri);
        }

        Ok(RouteSegment {
            string, index,
            name: segment.into(),
            kind: Kind::Static,
            _part: PhantomData
        })
    }

    pub fn parse_many(
        string: &'a str,
    ) -> impl Iterator<Item = SResult<'_, P>> {
        let mut last_multi_seg: Option<&str> = None;
        string.split(P::DELIMITER)
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(move |(i, seg)| {
                if let Some(multi_seg) = last_multi_seg {
                    return Err((seg, Trailing(multi_seg)));
                }

                let mut parsed = Self::parse_one(seg).map_err(|e| (seg, e))?;
                if parsed.kind == Kind::Multi {
                    last_multi_seg = Some(seg);
                }

                parsed.index = Some(i);
                Ok(parsed)
            })
    }
}

impl<'a> RouteSegment<'a, Path> {
    pub fn parse(uri: &'a Origin<'_>) -> impl Iterator<Item = SResult<'a, Path>> {
        Self::parse_many(uri.path())
    }
}

impl<'a> RouteSegment<'a, Query> {
    pub fn parse(uri: &'a Origin<'_>) -> Option<impl Iterator<Item = SResult<'a, Query>>> {
        uri.query().map(|q| Self::parse_many(q))
    }
}
