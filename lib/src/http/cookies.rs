use std::fmt;
use std::cell::RefMut;

pub use cookie::{Cookie, Key, CookieJar};
use cookie::{SameSite, Delta};

use http::Header;

pub enum Cookies<'a> {
    Jarred(RefMut<'a, CookieJar>, &'a Key),
    Empty(CookieJar)
}

impl<'a> Cookies<'a> {
    #[inline]
    pub(crate) fn new(jar: RefMut<'a, CookieJar>, key: &'a Key) -> Cookies<'a> {
        Cookies::Jarred(jar, key)
    }

    #[inline]
    pub(crate) fn empty() -> Cookies<'static> {
        Cookies::Empty(CookieJar::new())
    }

    #[inline(always)]
    pub(crate) fn parse_cookie(cookie_str: &str) -> Option<Cookie<'static>> {
        Cookie::parse_encoded(cookie_str).map(|c| c.into_owned()).ok()
    }

    pub fn get(&self, name: &str) -> Option<&Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.get(name),
            Cookies::Empty(_) => None
        }
    }

    pub fn get_private(&mut self, name: &str) -> Option<Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref mut jar, key) => jar.private(key).get(name),
            Cookies::Empty(_) => None
        }
    }

    pub fn add(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, _) = *self {
            jar.add(cookie)
        }
    }

    pub fn add_private(&mut self, mut cookie: Cookie<'static>) {
        cookie.set_http_only(true);

        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        if cookie.same_site().is_none() {
            cookie.set_same_site(SameSite::Strict);
        }

        if let Cookies::Jarred(ref mut jar, key) = *self {
            jar.private(key).add(cookie)
        }
    }

    pub fn remove(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, _) = *self {
            jar.remove(cookie)
        }
    }

    pub fn remove_private(&mut self, mut cookie: Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        if let Cookies::Jarred(ref mut jar, key) = *self {
            jar.private(key).remove(cookie)
        }
    }

    pub fn iter<'s>(&'s self) -> impl Iterator<Item=&'s Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.iter(),
            Cookies::Empty(ref jar) => jar.iter()
        }
    }

    pub(crate) fn delta(&self) -> Delta {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.delta(),
            Cookies::Empty(ref jar) => jar.delta()
        }
    }
}

impl<'a> fmt::Debug for Cookies<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Cookies::Jarred(ref jar, _) => write!(f, "{:?}", jar),
            Cookies::Empty(ref jar) => write!(f, "{:?}", jar)
        }
    }
}

impl<'a, 'c> From<&'a Cookie<'c>> for Header<'static> {
    fn from(cookie: &Cookie) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}
