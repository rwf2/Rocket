use std::convert::TryInto;

use crate::request::Request;
use crate::response::{Response, Responder};
use crate::http::uri::Uri;
use crate::http::Status;

/// An empty redirect response to a given URL.
///
/// This type simplifies returning a redirect response to the client.
///
/// # Usage
///
/// All constructors accept a generic type of `T: TryInto<Uri<'static>>`. Among
/// the candidate types are:
///
///   * `String`
///   * `&'static str`
///   * [`Origin`](crate::http::uri::Origin)
///   * [`Authority`](crate::http::uri::Authority)
///   * [`Absolute`](crate::http::uri::Absolute)
///   * [`Uri`](crate::http::uri::Uri)
///
/// Any non-`'static` strings must first be allocated using `.to_string()` or
/// similar before being passed to a `Redirect` constructor. When redirecting to
/// a route, _always_ use [`uri!`] to construct a valid [`Origin`]:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
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
///     Redirect::to(uri!(hello: name, age))
/// }
/// ```
///
/// [`Origin`]: crate::http::uri::Origin
/// [`uri!`]: ../../rocket_codegen/macro.uri.html
#[derive(Debug)]
pub struct Redirect(Status, Option<Uri<'static>>);

impl Redirect {
    /// Construct a temporary "see other" (303) redirect response. This is the
    /// typical response when redirecting a user to another page. This type of
    /// redirect indicates that the client should look elsewhere, but always via
    /// a `GET` request, for a given resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::response::Redirect;
    ///
    /// # let query = "foo";
    /// let redirect = Redirect::to("/other_url");
    /// let redirect = Redirect::to(format!("https://google.com/{}", query));
    /// ```
    pub fn to<U: TryInto<Uri<'static>>>(uri: U) -> Redirect {
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
   /// use rocket::response::Redirect;
   ///
   /// # let query = "foo";
   /// let redirect = Redirect::temporary("/other_url");
   /// let redirect = Redirect::temporary(format!("https://google.com/{}", query));
   /// ```
   pub fn temporary<U: TryInto<Uri<'static>>>(uri: U) -> Redirect {
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
   /// use rocket::response::Redirect;
   ///
   /// # let query = "foo";
   /// let redirect = Redirect::permanent("/other_url");
   /// let redirect = Redirect::permanent(format!("https://google.com/{}", query));
   /// ```
   pub fn permanent<U: TryInto<Uri<'static>>>(uri: U) -> Redirect {
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
   /// use rocket::response::Redirect;
   ///
   /// # let query = "foo";
   /// let redirect = Redirect::found("/other_url");
   /// let redirect = Redirect::found(format!("https://google.com/{}", query));
   /// ```
   pub fn found<U: TryInto<Uri<'static>>>(uri: U) -> Redirect {
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
   /// use rocket::response::Redirect;
   ///
   /// # let query = "foo";
   /// let redirect = Redirect::moved("/other_url");
   /// let redirect = Redirect::moved(format!("https://google.com/{}", query));
   /// ```
   pub fn moved<U: TryInto<Uri<'static>>>(uri: U) -> Redirect {
       Redirect(Status::MovedPermanently, uri.try_into().ok())
   }
}

/// Constructs a response with the appropriate status code and the given URL in
/// the `Location` header field. The body of the response is empty. If the URI
/// value used to create the `Responder` is an invalid URI, an error of
/// `Status::InternalServerError` is returned.
impl Responder<'_> for Redirect {
    fn respond_to(self, _: &Request<'_>) -> Result<Response<'static>, Status> {
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
