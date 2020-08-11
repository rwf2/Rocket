use std::marker::PhantomData;
use std::borrow::Cow;

use percent_encoding::{AsciiSet, utf8_percent_encode};

use crate::uri::{UriPart, Path, Query};
use crate::parse::uri::PATH_SET;

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct UNSAFE_ENCODE_SET<P: UriPart>(PhantomData<P>);
pub trait EncodeSet {
    const SET: AsciiSet;
}

impl<P: UriPart> Default for UNSAFE_ENCODE_SET<P> {
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
pub struct ENCODE_SET<P: UriPart>(PhantomData<P>);

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

pub fn unsafe_percent_encode<P: UriPart>(string: &str) -> Cow<'_, str> {
    match P::DELIMITER {
        '/' => percent_encode::<UNSAFE_ENCODE_SET<Path>>(string),
        '&' => percent_encode::<UNSAFE_ENCODE_SET<Query>>(string),
        _ => percent_encode::<DEFAULT_ENCODE_SET>(string)
    }
}

pub fn percent_encode<S: EncodeSet + Default>(string: &str) -> Cow<'_, str> {
    utf8_percent_encode(string, &S::SET).into()
}
