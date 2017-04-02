use http::Header;

use std::cell::RefMut;

pub use cookie::{Cookie, CookieJar, Iter, CookieBuilder, Delta};

use cookie::{PrivateJar, Key};

impl<'a, 'c> From<&'a Cookie<'c>> for Header<'static> {
    fn from(cookie: &Cookie) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

#[derive(Debug)]
pub enum Cookies<'a> {
    Jarred(RefMut<'a, CookieJar>),
    Empty(CookieJar)
}

impl<'a> Cookies<'a> {
    pub(crate) fn new(jar: RefMut<'a, CookieJar>) -> Cookies<'a> {
        Cookies::Jarred(jar)
    }

    pub(crate) fn empty() -> Cookies<'static> {
        Cookies::Empty(CookieJar::new())
    }

    #[inline(always)]
    pub(crate) fn parse_cookie(cookie_str: &str) -> Option<Cookie<'static>> {
        Cookie::parse_encoded(cookie_str).map(|c| c.into_owned()).ok()
    }

    pub fn get(&self, name: &str) -> Option<&Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref jar) => jar.get(name),
            Cookies::Empty(_) => None
        }
    }

    pub fn add(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar) = *self {
            jar.add(cookie)
        }
    }

    pub fn remove(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar) = *self {
            jar.remove(cookie)
        }
    }

    pub(crate) fn private(&mut self, key: &Key) -> PrivateJar {
        match *self {
            Cookies::Jarred(ref mut jar) => jar.private(key),
            Cookies::Empty(ref mut jar) => jar.private(key)
        }
    }

    pub fn iter(&self) -> Iter {
        match *self {
            Cookies::Jarred(ref jar) => jar.iter(),
            Cookies::Empty(ref jar) => jar.iter()
        }
    }

    pub(crate) fn delta(&self) -> Delta {
        match *self {
            Cookies::Jarred(ref jar) => jar.delta(),
            Cookies::Empty(ref jar) => jar.delta()
        }
    }
}
