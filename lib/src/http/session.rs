use std::cell::{RefCell, RefMut};

use time::{self, Duration};
use cookie::{Cookie, CookieJar, Delta};
pub use cookie::Key;

use http::{Header, Cookies};

const SESSION_PREFIX: &'static str = "__sess_";

pub struct Session<'a> {
    cookies: RefCell<Cookies<'a>>,
    key: &'a Key
}

impl<'a> Session<'a> {
    #[inline(always)]
    pub(crate) fn new(jar: RefMut<'a, CookieJar>, key: &'a Key) -> Session<'a> {
        Session { cookies: RefCell::new(Cookies::new(jar)), key: key }
    }

    #[inline(always)]
    pub(crate) fn empty(key: &'a Key) -> Session<'a> {
        Session { cookies: RefCell::new(Cookies::empty()), key: key }
    }

    #[inline(always)]
    pub(crate) fn header_for(cookie: &Cookie) -> Header<'static> {
        Header::new("Set-Cookie", format!("{}{}", SESSION_PREFIX, cookie))
    }

    #[inline(always)]
    pub(crate) fn parse_cookie(cookie_str: &str) -> Option<Cookie<'static>> {
        if !cookie_str.starts_with(SESSION_PREFIX) {
            return None;
        }

        Cookie::parse(&cookie_str[SESSION_PREFIX.len()..]).ok()
            .map(|c| c.into_owned())
    }

    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.cookies.borrow_mut().private(&self.key).get(name)
    }

    pub fn set(&mut self, mut cookie: Cookie<'static>) {
        cookie.set_http_only(true);

        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        // TODO: Should this be configurable?
        if cookie.max_age().is_none() && cookie.expires().is_none() {
            let session_lifetime = Duration::hours(3);
            cookie.set_max_age(session_lifetime);
            cookie.set_expires(time::now() + session_lifetime);
        }

        self.cookies.get_mut().private(&self.key).add(cookie)
    }

    pub fn remove(&mut self, cookie: Cookie<'static>) {
        self.cookies.get_mut().private(&self.key).remove(cookie)
    }

    #[inline(always)]
    pub(crate) fn delta(&mut self) -> Delta {
        self.cookies.get_mut().delta()
    }
}
