use std::fmt;
use std::ops::Deref;

use ref_cast::RefCast;

use crate::mtls::oid;
use crate::mtls::x509::X509Name;

/// An X.509 Distinguished Name (DN) found in a
/// [`Certificate`](crate::mtls::Certificate).
///
/// This type is a wrapper over [`X509Name`] with convenient methods and
/// complete documentation. Should the data exposed by the inherent methods not
/// suffice, this type derefs to [`X509Name`].
#[repr(transparent)]
#[derive(Debug, PartialEq, RefCast)]
pub struct Name<'a>(X509Name<'a>);

impl<'a> Name<'a> {
    /// Returns the _first_ UTF-8 _string_ common name, if any.
    ///
    /// Note that common names need not be UTF-8 strings, or strings at all.
    /// This method returns the first common name attribute that is.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket_community as rocket;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     if let Some(name) = cert.subject().common_name() {
    ///         println!("Hello, {}!", name);
    ///     }
    /// }
    /// ```
    pub fn common_name(&self) -> Option<&'a str> {
        self.common_names().next()
    }

    /// Returns an iterator over all of the UTF-8 _string_ common names in
    /// `self`.
    ///
    /// Note that common names need not be UTF-8 strings, or strings at all.
    /// This method filters the common names in `self` to those that are. Use
    /// the raw [`iter_common_name()`](#method.iter_common_name) to iterate over
    /// all value types.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket_community as rocket;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     for name in cert.issuer().common_names() {
    ///         println!("Issued by {}.", name);
    ///     }
    /// }
    /// ```
    pub fn common_names(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.iter_by_oid(&oid::OID_X509_COMMON_NAME)
            .filter_map(|n| n.as_str().ok())
    }

    /// Returns the _first_ UTF-8 _string_ email address, if any.
    ///
    /// Note that email addresses need not be UTF-8 strings, or strings at all.
    /// This method returns the first email address attribute that is.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket_community as rocket;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     if let Some(email) = cert.subject().email() {
    ///         println!("Hello, {}!", email);
    ///     }
    /// }
    /// ```
    pub fn email(&self) -> Option<&'a str> {
        self.emails().next()
    }

    /// Returns an iterator over all of the UTF-8 _string_ email addresses in
    /// `self`.
    ///
    /// Note that email addresses need not be UTF-8 strings, or strings at all.
    /// This method filters the email address in `self` to those that are. Use
    /// the raw [`iter_email()`](#method.iter_email) to iterate over all value
    /// types.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket_community as rocket;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     for email in cert.subject().emails() {
    ///         println!("Reach me at: {}", email);
    ///     }
    /// }
    /// ```
    pub fn emails(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.iter_by_oid(&oid::OID_PKCS9_EMAIL_ADDRESS)
            .filter_map(|n| n.as_str().ok())
    }

    /// Returns `true` if `self` has no data.
    ///
    /// When this is the case for a `subject()`, the subject data can be found
    /// in the `subjectAlt` [`extension`].
    ///
    /// [`extension`]: crate::mtls::Certificate::extensions()
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket_community as rocket;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     let no_data = cert.subject().is_empty();
    /// }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.as_raw().is_empty()
    }
}

impl<'a> Deref for Name<'a> {
    type Target = X509Name<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
