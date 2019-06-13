use std::fmt;
use std::borrow::Cow;

use rocket::request::FromParam;
use rocket::http::RawStr;
use rand::{self, Rng};

/// Table to retrieve base62 values from.
const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A _probably_ unique paste ID.
pub struct PasteID<'a>(Cow<'a, str>);

impl<'a> PasteID<'a> {
    /// Generate a _probably_ unique ID with `size` characters. For readability,
    /// the characters used are from the sets [0-9], [A-Z], [a-z]. The
    /// probability of a collision depends on the value of `size` and the number
    /// of IDs generated thus far.
    pub fn new(size: usize) -> PasteID<'static> {
        let mut id = String::with_capacity(size);
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            id.push(BASE62[rng.gen::<usize>() % 62] as char);
        }

        PasteID(Cow::Owned(id))
    }
}

impl<'a> fmt::Display for PasteID<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Returns `true` if `id` is a valid paste ID and `false` otherwise.
fn valid_id(id: &str) -> bool {
    id.chars().all(|c| {
        (c >= 'a' && c <= 'z')
            || (c >= 'A' && c <= 'Z')
            || (c >= '0' && c <= '9')
    })
}

/// Returns an instance of `PasteID` if the path segment is a valid ID.
/// Otherwise returns the invalid ID as the `Err` value.
impl<'a> FromParam<'a> for PasteID<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<PasteID<'a>, &'a RawStr> {
        match valid_id(param) {
            true => Ok(PasteID(Cow::Borrowed(param))),
            false => Err(param)
        }
    }
}

