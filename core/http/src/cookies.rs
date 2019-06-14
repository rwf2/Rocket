use std::fmt;
use std::cell::RefMut;

use crate::Header;
use cookie::Delta;

#[doc(hidden)] pub use self::key::*;
pub use cookie::{Cookie, CookieJar, SameSite};

/// Types and methods to manage a `Key` when private cookies are enabled.
#[cfg(feature = "private-cookies")]
mod key {
    pub use cookie::Key;
}

/// Types and methods to manage a `Key` when private cookies are disabled.
#[cfg(not(feature = "private-cookies"))]
mod key {
    #[derive(Copy, Clone)]
    pub struct Key;

    impl Key {
        pub fn generate() -> Self { Key }
        pub fn from_master(_bytes: &[u8]) -> Self { Key }
    }
}

/// Collection of one or more HTTP cookies.
///
/// The `Cookies` type allows for retrieval of cookies from an incoming request
/// as well as modifications to cookies to be reflected by Rocket on outgoing
/// responses. `Cookies` is a smart-pointer; it internally borrows and refers to
/// the collection of cookies active during a request's life-cycle.
///
/// # Usage
///
/// A type of `Cookies` can be retrieved via its `FromRequest` implementation as
/// a request guard or via the [`Request::cookies()`] method. Individual cookies
/// can be retrieved via the [`get()`] and [`get_private()`] methods. Cookies
/// can be added or removed via the [`add()`], [`add_private()`], [`remove()`],
/// and [`remove_private()`] methods.
///
/// [`Request::cookies()`]: rocket::Request::cookies()
/// [`get()`]: #method.get
/// [`get_private()`]: #method.get_private
/// [`add()`]: #method.add
/// [`add_private()`]: #method.add_private
/// [`remove()`]: #method.remove
/// [`remove_private()`]: #method.remove_private
///
/// ## Examples
///
/// The following short snippet shows `Cookies` being used as a request guard in
/// a handler to retrieve the value of a "message" cookie.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::http::Cookies;
///
/// #[get("/message")]
/// fn message(cookies: Cookies) -> Option<String> {
///     cookies.get("message").map(|c| format!("Message: {}", c.value()))
/// }
/// # fn main() {  }
/// ```
///
/// The following snippet shows `Cookies` being retrieved from a `Request` in a
/// custom request guard implementation for `User`. A [private cookie]
/// containing a user's ID is retrieved. If the cookie exists and the ID parses
/// as an integer, a `User` structure is validated. Otherwise, the guard
/// forwards.
///
/// [private cookie]: Cookies::add_private()
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro, never_type)]
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::http::Status;
/// use rocket::outcome::IntoOutcome;
/// use rocket::request::{self, Request, FromRequest};
///
/// // In practice, we'd probably fetch the user from the database.
/// struct User(usize);
///
/// impl FromRequest<'_, '_> for User {
///     type Error = !;
///
///     fn from_request(request: &Request<'_>) -> request::Outcome<User, !> {
///         request.cookies()
///             .get_private("user_id")
///             .and_then(|cookie| cookie.value().parse().ok())
///             .map(|id| User(id))
///             .or_forward(())
///     }
/// }
/// # fn main() { }
/// ```
///
/// # Private Cookies
///
/// _Private_ cookies are just like regular cookies except that they are
/// encrypted using authenticated encryption, a form of encryption which
/// simultaneously provides confidentiality, integrity, and authenticity. This
/// means that private cookies cannot be inspected, tampered with, or
/// manufactured by clients. If you prefer, you can think of private cookies as
/// being signed and encrypted.
///
/// Private cookies can be retrieved, added, and removed from a `Cookies`
/// collection via the [`get_private()`], [`add_private()`], and
/// [`remove_private()`] methods.
///
/// ## Encryption Key
///
/// To encrypt private cookies, Rocket uses the 256-bit key specified in the
/// `secret_key` configuration parameter. If one is not specified, Rocket will
/// automatically generate a fresh key. Note, however, that a private cookie can
/// only be decrypted with the same key with which it was encrypted. As such, it
/// is important to set a `secret_key` configuration parameter when using
/// private cookies so that cookies decrypt properly after an application
/// restart. Rocket will emit a warning if an application is run in production
/// mode without a configured `secret_key`.
///
/// Generating a string suitable for use as a `secret_key` configuration value
/// is usually done through tools like `openssl`. Using `openssl`, for instance,
/// a 256-bit base64 key can be generated with the command `openssl rand -base64
/// 32`.
pub enum Cookies<'a> {
    #[doc(hidden)]
    Jarred(RefMut<'a, CookieJar>, &'a Key),
    #[doc(hidden)]
    Empty(CookieJar)
}

impl<'a> Cookies<'a> {
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[inline]
    #[doc(hidden)]
    pub fn new(jar: RefMut<'a, CookieJar>, key: &'a Key) -> Cookies<'a> {
        Cookies::Jarred(jar, key)
    }

    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    #[inline(always)]
    pub fn empty() -> Cookies<'static> {
        Cookies::Empty(CookieJar::new())
    }

    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    #[inline(always)]
    pub fn parse_cookie(cookie_str: &str) -> Option<Cookie<'static>> {
        Cookie::parse_encoded(cookie_str).map(|c| c.into_owned()).ok()
    }

    /// Adds an original `cookie` to this collection.
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[inline]
    #[doc(hidden)]
    pub fn add_original(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, _) = *self {
            jar.add_original(cookie)
        }
    }

    /// Returns a reference to the `Cookie` inside this container with the name
    /// `name`. If no such cookie exists, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Cookies;
    ///
    /// fn handler(cookies: Cookies) {
    ///     let cookie = cookies.get("name");
    /// }
    /// ```
    pub fn get(&self, name: &str) -> Option<&Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.get(name),
            Cookies::Empty(_) => None
        }
    }

    /// Adds `cookie` to this collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{Cookie, Cookies};
    ///
    /// fn handler(mut cookies: Cookies) {
    ///     cookies.add(Cookie::new("name", "value"));
    ///
    ///     let cookie = Cookie::build("name", "value")
    ///         .path("/")
    ///         .secure(true)
    ///         .finish();
    ///
    ///     cookies.add(cookie);
    /// }
    /// ```
    pub fn add(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, _) = *self {
            jar.add(cookie)
        }
    }

    /// Removes `cookie` from this collection and generates a "removal" cookies
    /// to send to the client on response. For correctness, `cookie` must
    /// contain the same `path` and `domain` as the cookie that was initially
    /// set. Failure to provide the initial `path` and `domain` will result in
    /// cookies that are not properly removed.
    ///
    /// A "removal" cookie is a cookie that has the same name as the original
    /// cookie but has an empty value, a max-age of 0, and an expiration date
    /// far in the past.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{Cookie, Cookies};
    ///
    /// fn handler(mut cookies: Cookies) {
    ///     cookies.remove(Cookie::named("name"));
    /// }
    /// ```
    pub fn remove(&mut self, cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, _) = *self {
            jar.remove(cookie)
        }
    }

    /// Returns an iterator over all of the cookies present in this collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Cookies;
    ///
    /// fn handler(cookies: Cookies) {
    ///     for c in cookies.iter() {
    ///         println!("Name: '{}', Value: '{}'", c.name(), c.value());
    ///     }
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item=&Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.iter(),
            Cookies::Empty(ref jar) => jar.iter()
        }
    }

    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[inline]
    #[doc(hidden)]
    pub fn delta(&self) -> Delta<'_> {
        match *self {
            Cookies::Jarred(ref jar, _) => jar.delta(),
            Cookies::Empty(ref jar) => jar.delta()
        }
    }
}

#[cfg(feature = "private-cookies")]
impl Cookies<'_> {
    /// Returns a reference to the `Cookie` inside this collection with the name
    /// `name` and authenticates and decrypts the cookie's value, returning a
    /// `Cookie` with the decrypted value. If the cookie cannot be found, or the
    /// cookie fails to authenticate or decrypt, `None` is returned.
    ///
    /// This method is only available when the `private-cookies` feature is
    /// enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Cookies;
    ///
    /// fn handler(mut cookies: Cookies) {
    ///     let cookie = cookies.get_private("name");
    /// }
    /// ```
    pub fn get_private(&mut self, name: &str) -> Option<Cookie<'static>> {
        match *self {
            Cookies::Jarred(ref mut jar, key) => jar.private(key).get(name),
            Cookies::Empty(_) => None
        }
    }

    /// Adds `cookie` to the collection. The cookie's value is encrypted with
    /// authenticated encryption assuring confidentiality, integrity, and
    /// authenticity. The cookie can later be retrieved using
    /// [`get_private`](#method.get_private) and removed using
    /// [`remove_private`](#method.remove_private).
    ///
    /// Unless a value is supplied for the given key, the following defaults are
    /// set on `cookie` before being added to `self`:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///    * `HttpOnly`: `true`
    ///    * `Expires`: 1 week from now
    ///
    /// These defaults ensure maximum usability and security. For additional
    /// security, you may wish to set the `secure` flag.
    ///
    /// This method is only available when the `private-cookies` feature is
    /// enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{Cookie, Cookies};
    ///
    /// fn handler(mut cookies: Cookies) {
    ///     cookies.add_private(Cookie::new("name", "value"));
    /// }
    /// ```
    pub fn add_private(&mut self, mut cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, key) = *self {
            Cookies::set_private_defaults(&mut cookie);
            jar.private(key).add(cookie)
        }
    }

    /// Adds an original, private `cookie` to the collection.
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn add_original_private(&mut self, mut cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, key) = *self {
            Cookies::set_private_defaults(&mut cookie);
            jar.private(key).add_original(cookie)
        }
    }

    /// For each property mentioned below, this method checks
    /// if there is a provided value and if there is none, sets
    /// a default value.
    /// Default values are:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///    * `HttpOnly`: `true`
    ///    * `Expires`: 1 week from now
    ///
    fn set_private_defaults(cookie: &mut Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        if cookie.http_only().is_none() {
            cookie.set_http_only(true);
        }

        if cookie.expires().is_none() {
            cookie.set_expires(time::now() + time::Duration::weeks(1));
        }

        if cookie.same_site().is_none() {
            cookie.set_same_site(SameSite::Strict);
        }
    }

    /// Removes the private `cookie` from the collection.
    ///
    /// For correct removal, the passed in `cookie` must contain the same `path`
    /// and `domain` as the cookie that was initially set. If a path is not set
    /// on `cookie`, the `"/"` path will automatically be set.
    ///
    /// This method is only available when the `private-cookies` feature is
    /// enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{Cookie, Cookies};
    ///
    /// fn handler(mut cookies: Cookies) {
    ///     cookies.remove_private(Cookie::named("name"));
    /// }
    /// ```
    pub fn remove_private(&mut self, mut cookie: Cookie<'static>) {
        if let Cookies::Jarred(ref mut jar, key) = *self {
            if cookie.path().is_none() {
                cookie.set_path("/");
            }

            jar.private(key).remove(cookie)
        }
    }
}

impl fmt::Debug for Cookies<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cookies::Jarred(ref jar, _) => write!(f, "{:?}", jar),
            Cookies::Empty(ref jar) => write!(f, "{:?}", jar)
        }
    }
}

impl From<Cookie<'_>> for Header<'static> {
    fn from(cookie: Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

impl From<&Cookie<'_>> for Header<'static> {
    fn from(cookie: &Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}
