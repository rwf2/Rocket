use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::uri::Reference;
use crate::http::Status;

/// An empty redirect response to a given URL.
///
/// This type simplifies returning a redirect response to the client.
///
/// # Usage
///
/// All constructors accept a generic type of `T: TryInto<Reference<'static>>`.
/// Among the candidate types are:
///
///   * `String`, `&'static str`
///   * [`Origin`](crate::http::uri::Origin)
///   * [`Authority`](crate::http::uri::Authority)
///   * [`Absolute`](crate::http::uri::Absolute)
///   * [`Reference`](crate::http::uri::Reference)
///
/// Any non-`'static` strings must first be allocated using `.to_string()` or
/// similar before being passed to a `Redirect` constructor. When redirecting to
/// a route, or any URI containing a route, _always_ use [`uri!`] to construct a
/// valid URI:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::response::Redirect;
///
/// #[get("/hello/<name>/<age>")]
/// fn hello(name: String, age: u8) -> String {
///     format!("Hello, {} year old named {}!", age, name)
/// }
///
/// #[get("/hi/<name>/<age>")]
/// fn hi(name: String, age: u8) -> Redirect {
///     Redirect::to(uri!(hello(name, age)))
/// }
///
/// #[get("/bye/<name>/<age>")]
/// fn bye(name: String, age: u8) -> Redirect {
///     Redirect::to(uri!("https://rocket.rs/bye", hello(name, age), "?bye#now"))
/// }
/// ```
///
/// [`Origin`]: crate::http::uri::Origin
/// [`uri!`]: ../macro.uri.html
#[derive(Debug)]
pub struct Redirect(Status, Option<Reference<'static>>);

impl Redirect {
    /// Construct a temporary "see other" (303) redirect response. This is the
    /// typical response when redirecting a user to another page. This type of
    /// redirect indicates that the client should look elsewhere, but always via
    /// a `GET` request, for a given resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::response::Redirect;
    ///
    /// let redirect = Redirect::to(uri!("/foo/bar"));
    /// let redirect = Redirect::to(uri!("https://domain.com#foo"));
    /// ```
    pub fn to<U: TryInto<Reference<'static>>>(uri: U) -> Redirect {
        Redirect(Status::SeeOther, uri.try_into().ok())
    }

    /// Construct a "temporary" (307) redirect response. This response instructs
    /// the client to reissue the current request to a different URL,
    /// maintaining the contents of the request identically. This means that,
    /// for example, a `POST` request will be resent, contents included, to the
    /// requested URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::response::Redirect;
    ///
    /// let redirect = Redirect::temporary(uri!("some/other/path"));
    /// let redirect = Redirect::temporary(uri!("https://rocket.rs?foo"));
    /// let redirect = Redirect::temporary(format!("some-{}-thing", "crazy"));
    /// ```
    pub fn temporary<U: TryInto<Reference<'static>>>(uri: U) -> Redirect {
        Redirect(Status::TemporaryRedirect, uri.try_into().ok())
    }

   /// Construct a "permanent" (308) redirect response. This redirect must only
   /// be used for permanent redirects as it is cached by clients. This
   /// response instructs the client to reissue requests for the current URL to
   /// a different URL, now and in the future, maintaining the contents of the
   /// request identically. This means that, for example, a `POST` request will
   /// be resent, contents included, to the requested URL.
   ///
   /// # Examples
   ///
   /// ```rust
   /// # #[macro_use] extern crate rocket;
   /// use rocket::response::Redirect;
   ///
   /// let redirect = Redirect::permanent(uri!("/other_url"));
   /// let redirect = Redirect::permanent(format!("some-{}-thing", "crazy"));
   /// ```
   pub fn permanent<U: TryInto<Reference<'static>>>(uri: U) -> Redirect {
       Redirect(Status::PermanentRedirect, uri.try_into().ok())
   }

   /// Construct a temporary "found" (302) redirect response. This response
   /// instructs the client to reissue the current request to a different URL,
   /// ideally maintaining the contents of the request identically.
   /// Unfortunately, different clients may respond differently to this type of
   /// redirect, so `303` or `307` redirects, which disambiguate, are
   /// preferred.
   ///
   /// # Examples
   ///
   /// ```rust
   /// # #[macro_use] extern crate rocket;
   /// use rocket::response::Redirect;
   ///
   /// let redirect = Redirect::found(uri!("/other_url"));
   /// let redirect = Redirect::found(format!("some-{}-thing", "crazy"));
   /// ```
   pub fn found<U: TryInto<Reference<'static>>>(uri: U) -> Redirect {
       Redirect(Status::Found, uri.try_into().ok())
   }

   /// Construct a permanent "moved" (301) redirect response. This response
   /// should only be used for permanent redirects as it can be cached by
   /// browsers. Because different clients may respond differently to this type
   /// of redirect, a `308` redirect, which disambiguates, is preferred.
   ///
   /// # Examples
   ///
   /// ```rust
   /// # #[macro_use] extern crate rocket;
   /// use rocket::response::Redirect;
   ///
   /// let redirect = Redirect::moved(uri!("here"));
   /// let redirect = Redirect::moved(format!("some-{}-thing", "crazy"));
   /// ```
   pub fn moved<U: TryInto<Reference<'static>>>(uri: U) -> Redirect {
       Redirect(Status::MovedPermanently, uri.try_into().ok())
   }
}

/// Constructs a response with the appropriate status code and the given URL in
/// the `Location` header field. The body of the response is empty. If the URI
/// value used to create the `Responder` is an invalid URI, an error of
/// `Status::InternalServerError` is returned.
impl<'r> Responder<'r, 'static> for Redirect {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        if let Some(uri) = self.1 {
            Response::build()
                .status(self.0)
                .raw_header("Location", uri.to_string())
                .ok()
        } else {
            error!("Invalid URI used for redirect.");
            Err(Status::InternalServerError)
        }
    }
}
