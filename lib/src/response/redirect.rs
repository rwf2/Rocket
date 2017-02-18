use response::{Response, Responder};
use http::hyper::header;
use http::Status;

/// An empty redirect response to a given URL.
///
/// This type simplifies returning a redirect response to the client.
#[derive(Debug)]
pub struct Redirect(Status, String);

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
    /// # #[allow(unused_variables)]
    /// let redirect = Redirect::to("/other_url");
    /// ```
    pub fn to(uri: &str) -> Redirect {
        Redirect(Status::SeeOther, String::from(uri))
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
    /// # #[allow(unused_variables)]
    /// let redirect = Redirect::temporary("/other_url");
    /// ```
    pub fn temporary(uri: &str) -> Redirect {
        Redirect(Status::TemporaryRedirect, String::from(uri))
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
    /// # #[allow(unused_variables)]
    /// let redirect = Redirect::permanent("/other_url");
    /// ```
    pub fn permanent(uri: &str) -> Redirect {
        Redirect(Status::PermanentRedirect, String::from(uri))
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
    /// # #[allow(unused_variables)]
    /// let redirect = Redirect::found("/other_url");
    /// ```
    pub fn found(uri: &str) -> Redirect {
        Redirect(Status::Found, String::from(uri))
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
    /// # #[allow(unused_variables)]
    /// let redirect = Redirect::moved("/other_url");
    /// ```
    pub fn moved(uri: &str) -> Redirect {
        Redirect(Status::MovedPermanently, String::from(uri))
    }
}

/// Constructs a response with the appropriate status code and the given URL in
/// the `Location` header field. The body of the response is empty. This
/// responder does not fail.
impl Responder<'static> for Redirect {
    fn respond(self) -> Result<Response<'static>, Status> {
        Response::build()
            .status(self.0)
            .header(header::Location(self.1))
            .ok()
    }
}
