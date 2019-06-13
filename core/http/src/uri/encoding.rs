use std::marker::PhantomData;
use std::borrow::Cow;

use percent_encoding::{EncodeSet, utf8_percent_encode};

use crate::uri::{UriPart, Path, Query};
use crate::parse::uri::is_pchar;

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
crate struct UNSAFE_ENCODE_SET<P: UriPart>(PhantomData<P>);

impl<P: UriPart> Default for UNSAFE_ENCODE_SET<P> {
    #[inline(always)]
    fn default() -> Self { UNSAFE_ENCODE_SET(PhantomData) }
}

impl EncodeSet for UNSAFE_ENCODE_SET<Path> {
    #[inline(always)]
    fn contains(&self, byte: u8) -> bool {
        !is_pchar(byte) || byte == b'%'
    }
}

impl EncodeSet for UNSAFE_ENCODE_SET<Query> {
    #[inline(always)]
    fn contains(&self, byte: u8) -> bool {
        (!is_pchar(byte) && (byte != b'?')) || byte == b'%' || byte == b'+'
    }
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
crate struct ENCODE_SET<P: UriPart>(PhantomData<P>);

impl EncodeSet for ENCODE_SET<Path> {
    #[inline(always)]
    fn contains(&self, byte: u8) -> bool {
        <UNSAFE_ENCODE_SET<Path>>::default().contains(byte) || byte == b'/'
    }
}

impl EncodeSet for ENCODE_SET<Query> {
    #[inline(always)]
    fn contains(&self, byte: u8) -> bool {
        <UNSAFE_ENCODE_SET<Query>>::default().contains(byte) || match byte {
            b'&' | b'=' => true,
            _ => false
        }
    }
}

#[derive(Default, Clone, Copy)]
#[allow(non_camel_case_types)]
crate struct DEFAULT_ENCODE_SET;

impl EncodeSet for DEFAULT_ENCODE_SET {
    #[inline(always)]
    fn contains(&self, byte: u8) -> bool {
        ENCODE_SET::<Path>(PhantomData).contains(byte) ||
            ENCODE_SET::<Query>(PhantomData).contains(byte)
    }
}

crate fn unsafe_percent_encode<P: UriPart>(string: &str) -> Cow<'_, str> {
    match P::DELIMITER {
        '/' => percent_encode::<UNSAFE_ENCODE_SET<Path>>(string),
        '&' => percent_encode::<UNSAFE_ENCODE_SET<Query>>(string),
        _ => percent_encode::<DEFAULT_ENCODE_SET>(string)
    }
}

crate fn percent_encode<S: EncodeSet + Default>(string: &str) -> Cow<'_, str> {
    utf8_percent_encode(string, S::default()).into()
}
