use std::marker::PhantomData;
use std::borrow::Cow;

use percent_encoding::AsciiSet;

use crate::RawStr;
use crate::uri::fmt::{Part, Path, Query};
use crate::parse::uri::tables::PATH_CHARS;

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct UNSAFE_ENCODE_SET<P: Part>(PhantomData<P>);

pub trait EncodeSet {
    const SET: AsciiSet;
}

const fn set_from_table(table: &'static [u8; 256]) -> AsciiSet {
    const ASCII_RANGE_LEN: u8 = 0x80;

    let mut set = percent_encoding::CONTROLS.add(0);
    let mut i: u8 = 0;
    while i < ASCII_RANGE_LEN {
        if table[i as usize] == 0 {
            set = set.add(i);
        }

        i += 1;
    }

    set
}

const PATH_SET: AsciiSet = set_from_table(&PATH_CHARS);

impl<P: Part> Default for UNSAFE_ENCODE_SET<P> {
    #[inline(always)]
    fn default() -> Self { UNSAFE_ENCODE_SET(PhantomData) }
}

impl EncodeSet for UNSAFE_ENCODE_SET<Path> {
    const SET: AsciiSet = PATH_SET
        .add(b'%');
}

impl EncodeSet for UNSAFE_ENCODE_SET<Query> {
    const SET: AsciiSet = PATH_SET
        .remove(b'?')
        .add(b'%')
        .add(b'+');
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct ENCODE_SET<P: Part>(PhantomData<P>);

impl EncodeSet for ENCODE_SET<Path> {
    const SET: AsciiSet = <UNSAFE_ENCODE_SET<Path>>::SET
        .add(b'/');
}

impl EncodeSet for ENCODE_SET<Query> {
    const SET: AsciiSet = <UNSAFE_ENCODE_SET<Query>>::SET
        .add(b'&')
        .add(b'=');
}

#[derive(Default, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct DEFAULT_ENCODE_SET;

impl EncodeSet for DEFAULT_ENCODE_SET {
    // DEFAULT_ENCODE_SET Includes:
    // * ENCODE_SET<Path> (and UNSAFE_ENCODE_SET<Path>)
    const SET: AsciiSet = <ENCODE_SET<Path>>::SET
        // * UNSAFE_ENCODE_SET<Query>
        .add(b'%')
        .add(b'+')
        // * ENCODE_SET<Query>
        .add(b'&')
        .add(b'=');
}

pub fn percent_encode<S: EncodeSet + Default>(string: &RawStr) -> Cow<'_, str> {
    percent_encoding::utf8_percent_encode(string.as_str(), &S::SET).into()
}

pub fn percent_encode_bytes<S: EncodeSet + Default>(bytes: &[u8]) -> Cow<'_, str> {
    percent_encoding::percent_encode(bytes, &S::SET).into()
}
