use std::fmt::{self, Display};

use crate::uncased::UncasedStr;
use crate::uri::error::Error;
use crate::uri::{Absolute, Authority};

/// A domain and port identified by a client as the server being messaged.
///
/// For requests made via HTTP/1.1, a host is identified via the `HOST` header.
/// In HTTP/2 and HTTP/3, this information is instead communicated via the
/// `:authority` and `:port` pseudo-header request fields. It is a
/// client-controlled value via which the client communicates to the server the
/// domain name and port it is attemping to communicate with. The following
/// diagram illustrates the syntactic structure of a `Host`:
///
/// ```text
/// some.domain.foo:8088
/// |-----------| |--|
///     domain    port
/// ```
///
/// Only the domain part is required. Its value is case-insensitive.
///
/// # URI Construction
///
/// A `Host` is _not_ a [`Uri`](crate::uri::Uri), and none of Rocket's APIs will
/// accept a `Host` value as such. This is because doing so would facilitate the
/// construction of URIs to internal routes in a manner controllable by an
/// attacker, inevitably leading to "HTTP Host header attacks".
///
/// Instead, a `Host` must be checked before being converted to a [`Uri`]
/// value. The [`Host::to_authority`] and [`Host::to_absolute`] methods provide
/// these mechanisms:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type Token = String;
/// use rocket::http::uri::Host;
///
/// // A sensitive URI we want to prefix with safe hosts.
/// #[get("/token?<secret>")]
/// fn token(secret: Token) { /* .. */ }
///
/// // Whitelist of known hosts. In a real setting, you might retrieve this
/// // list from config at ignite-time using tools like `AdHoc::config()`.
/// const WHITELIST: [Host<'static>; 4] = [
///     Host::new(uri!("rocket.rs")),
///     Host::new(uri!("rocket.rs:443")),
///     Host::new(uri!("guide.rocket.rs")),
///     Host::new(uri!("guide.rocket.rs:443")),
/// ];
///
/// // Use `Host::to_absolute()` to case-insensitively check a host against a
/// // whitelist, returning an `Absolute` usable as a `uri!()` prefix.
/// let host = Host::new(uri!("guide.ROCKET.rs"));
/// let prefix = host.to_absolute("https", &WHITELIST);
///
/// // Since `guide.rocket.rs` is in the whitelist, `prefix` is `Some`.
/// assert!(prefix.is_some());
/// if let Some(prefix) = prefix {
///     // We can use this prefix to safely construct URIs.
///     let uri = uri!(prefix, token("some-secret-token"));
///     assert_eq!(uri, "https://guide.ROCKET.rs/token?secret=some-secret-token");
/// }
/// ```
///
/// # (De)serialization
///
/// `Host` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Host;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Host<'static>,
/// }
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriBorrowed<'a> {
///     uri: Host<'a>,
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Host<'a>(Authority<'a>);

impl<'a> Host<'a> {
    /// Create a new `Host` from an `Authority`. Only the `host` and `port`
    /// parts are preserved.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// let host = Host::new(uri!("developer.mozilla.org"));
    /// assert_eq!(host.to_string(), "developer.mozilla.org");
    ///
    /// let host = Host::new(uri!("foo:bar@developer.mozilla.org:1234"));
    /// assert_eq!(host.to_string(), "developer.mozilla.org:1234");
    ///
    /// let host = Host::new(uri!("rocket.rs:443"));
    /// assert_eq!(host.to_string(), "rocket.rs:443");
    /// ```
    pub const fn new(authority: Authority<'a>) -> Self {
        Host(authority)
    }

    /// Parses the string `string` into a `Host`. Parsing will never allocate.
    /// Returns an `Error` if `string` is not a valid authority URI, meaning
    /// that this parser accepts a `user_info` part for compatability but
    /// discards it.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// // Parse from a valid authority URI.
    /// let host = Host::parse("user:pass@domain").expect("valid host");
    /// assert_eq!(host.domain(), "domain");
    /// assert_eq!(host.port(), None);
    ///
    /// // Parse from a valid host.
    /// let host = Host::parse("domain:311").expect("valid host");
    /// assert_eq!(host.domain(), "doMaIN");
    /// assert_eq!(host.port(), Some(311));
    ///
    /// // Invalid hosts fail to parse.
    /// Host::parse("https://rocket.rs").expect_err("invalid host");
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let host = Host::new(uri!("domain"));
    /// assert_eq!(host.domain(), "domain");
    /// assert_eq!(host.port(), None);
    /// ```
    pub fn parse(string: &'a str) -> Result<Host<'a>, Error<'a>> {
        Host::parse_bytes(string.as_bytes())
    }

    /// PRIVATE: Used by core.
    #[doc(hidden)]
    pub fn parse_bytes(bytes: &'a [u8]) -> Result<Host<'a>, Error<'a>> {
        crate::parse::uri::authority_from_bytes(bytes).map(Host::new)
    }

    /// Parses the string `string` into an `Host`. Parsing never allocates
    /// on success. May allocate on error.
    ///
    /// This method should be used instead of [`Host::parse()`] when the source
    /// is already a `String`. Returns an `Error` if `string` is not a valid
    /// authority URI, meaning that this parser accepts a `user_info` part for
    /// compatability but discards it.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// let source = format!("rocket.rs:8000");
    /// let host = Host::parse_owned(source).expect("valid host");
    /// assert_eq!(host.domain(), "rocket.rs");
    /// assert_eq!(host.port(), Some(8000));
    /// ```
    pub fn parse_owned(string: String) -> Result<Host<'static>, Error<'static>> {
        Authority::parse_owned(string).map(Host::new)
    }

    /// Returns the case-insensitive domain part of the host.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// let host = Host::new(uri!("domain.com:123"));
    /// assert_eq!(host.domain(), "domain.com");
    ///
    /// let host = Host::new(uri!("username:password@domain:123"));
    /// assert_eq!(host.domain(), "domain");
    ///
    /// let host = Host::new(uri!("[1::2]:123"));
    /// assert_eq!(host.domain(), "[1::2]");
    /// ```
    #[inline]
    pub fn domain(&self) -> &UncasedStr {
        self.0.host().into()
    }

    /// Returns the port part of the host, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// // With a port.
    /// let host = Host::new(uri!("domain:123"));
    /// assert_eq!(host.port(), Some(123));
    ///
    /// let host = Host::new(uri!("domain.com:8181"));
    /// assert_eq!(host.port(), Some(8181));
    ///
    /// // Without a port.
    /// let host = Host::new(uri!("domain.foo.bar.tld"));
    /// assert_eq!(host.port(), None);
    /// ```
    #[inline(always)]
    pub fn port(&self) -> Option<u16> {
        self.0.port()
    }

    /// Checks `self` against `whitelist`. If `self` is in `whitelist`, returns
    /// an [`Authority`] URI representing self. Otherwise, returns `None`.
    /// Domain comparison is case-insensitive.
    ///
    /// See [URI construction](Self#uri-construction) for more.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// let whitelist = &[Host::new(uri!("domain.tld"))];
    ///
    /// // A host in the whitelist returns `Some`.
    /// let host = Host::new(uri!("domain.tld"));
    /// let uri = host.to_authority(whitelist);
    /// assert!(uri.is_some());
    /// assert_eq!(uri.unwrap().to_string(), "domain.tld");
    ///
    /// let host = Host::new(uri!("foo:bar@doMaIN.tLd"));
    /// let uri = host.to_authority(whitelist);
    /// assert!(uri.is_some());
    /// assert_eq!(uri.unwrap().to_string(), "doMaIN.tLd");
    ///
    /// // A host _not_ in the whitelist returns `None`.
    /// let host = Host::new(uri!("domain.tld:1234"));
    /// let uri = host.to_authority(whitelist);
    /// assert!(uri.is_none());
    /// ```
    pub fn to_authority<'h, W>(&self, whitelist: W) -> Option<Authority<'a>>
        where W: IntoIterator<Item = &'h Host<'h>>
    {
        let mut auth = whitelist.into_iter().any(|h| h == self).then(|| self.0.clone())?;
        auth.user_info = None;
        Some(auth)
    }

    /// Checks `self` against `whitelist`. If `self` is in `whitelist`, returns
    /// an [`Absolute`] URI representing `self` with scheme `scheme`. Otherwise,
    /// returns `None`. Domain comparison is case-insensitive.
    ///
    /// See [URI construction](Self#uri-construction) for more.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Host;
    ///
    /// let whitelist = &[Host::new(uri!("domain.tld:443"))];
    ///
    /// // A host in the whitelist returns `Some`.
    /// let host = Host::new(uri!("user@domain.tld:443"));
    /// let uri = host.to_absolute("http", whitelist);
    /// assert!(uri.is_some());
    /// assert_eq!(uri.unwrap().to_string(), "http://domain.tld:443");
    ///
    /// let host = Host::new(uri!("domain.TLD:443"));
    /// let uri = host.to_absolute("https", whitelist);
    /// assert!(uri.is_some());
    /// assert_eq!(uri.unwrap().to_string(), "https://domain.TLD:443");
    ///
    /// // A host _not_ in the whitelist returns `None`.
    /// let host = Host::new(uri!("domain.tld"));
    /// let uri = host.to_absolute("http", whitelist);
    /// assert!(uri.is_none());
    /// ```
    pub fn to_absolute<'h, W>(&self, scheme: &'a str, whitelist: W) -> Option<Absolute<'a>>
        where W: IntoIterator<Item = &'h Host<'h>>
    {
        let scheme = crate::parse::uri::scheme_from_str(scheme).ok()?;
        let authority = self.to_authority(whitelist)?;
        Some(Absolute::const_new(scheme, Some(authority), "", None))
    }
}

impl_serde!(Host<'a>, "an HTTP host");

impl_base_traits!(Host, domain, port);

impl crate::ext::IntoOwned for Host<'_> {
    type Owned = Host<'static>;

    fn into_owned(self) -> Host<'static> {
        Host(self.0.into_owned())
    }
}

impl<'a> From<Authority<'a>> for Host<'a> {
    fn from(auth: Authority<'a>) -> Self {
        Host::new(auth)
    }
}

impl Display for Host<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.domain().fmt(f)?;
        if let Some(port) = self.port() {
            write!(f, ":{}", port)?;
        }

        Ok(())
    }
}
