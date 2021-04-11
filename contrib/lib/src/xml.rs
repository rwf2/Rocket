//! Automatic Xml (de)serialization support.
//!
//! See the [`Xml`](crate::xml::Xml) type for further details.
//!
//! # Enabling
//!
//! This module is only available when the `xml` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["xml"]
//! ```

use std::io;
use std::ops::{Deref, DerefMut};

use rocket::request::Request;
use rocket::data::{ByteUnit, Data, FromData, Outcome};
use rocket::response::{self, Responder, content};
use rocket::http::Status;
use rocket::form::prelude as form;

use serde::Serialize;
use serde::de::DeserializeOwned;

#[doc(hidden)]
pub use quick_xml::DeError as Error;
use quick_xml::Error as XmlError;

/// The XML data guard: easily consume and respond with XML.
///
/// ## Receiving XML
///
/// `Xml` is both a data guard and a form guard.
///
/// ### Data Guard
///
/// To parse request body data as XML , add a `data` route argument with a
/// target type of `Xml<T>`, where `T` is some type you'd like to parse from
/// XML. `T` must implement [`serde::Deserialize`].
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// use rocket_contrib::xml::Xml;
///
/// #[post("/user", format = "xml", data = "<user>")]
/// fn new_user(user: Xml<User>) {
///     /* ... */
/// }
/// ```
///
/// You don't _need_ to use `format = "xml"`, but it _may_ be what you want.
/// Using `format = xml` means that any request that doesn't specify
/// "text/xml" as its `Content-Type` header value will not be routed to
/// the handler.
///
/// ### Form Guard
///
/// `Xml<T>`, as a form guard, accepts value and data fields and parses the
/// data as a `T`. Simple use `Xml<T>`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # extern crate rocket_contrib;
/// # type Metadata = usize;
/// use rocket::form::{Form, FromForm};
/// use rocket_contrib::xml::Xml;
///
/// #[derive(FromForm)]
/// struct User<'r> {
///     name: &'r str,
///     metadata: Xml<Metadata>
/// }
///
/// #[post("/user", data = "<form>")]
/// fn new_user(form: Form<User<'_>>) {
///     /* ... */
/// }
/// ```
///
/// ## Sending XML
///
/// If you're responding with XML data, return a `Xml<T>` type, where `T`
/// implements [`Serialize`] from [`serde`]. The content type of the response is
/// set to `text/xml` automatically.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// use rocket_contrib::xml::Xml;
///
/// #[get("/users/<id>")]
/// fn user(id: usize) -> Xml<User> {
///     let user_from_id = User::from(id);
///     /* ... */
///     Xml(user_from_id)
/// }
/// ```
///
/// ## Incoming Data Limits
///
/// The default size limit for incoming XML data is 1MiB. Setting a limit
/// protects your application from denial of service (DoS) attacks and from
/// resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.xml` configuration parameter. For
/// instance, to increase the XML limit to 5MiB for all environments, you may
/// add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// xml = 5242880
/// ```
#[derive(Debug)]
pub struct Xml<T>(pub T);

const DEFAULT_LIMIT: ByteUnit = ByteUnit::Mebibyte(1);

impl<T> Xml<T> {
    /// Consumes the XML wrapper and returns the wrapped item.
    ///
    /// # Example
    /// ```rust
    /// # use rocket_contrib::xml::Xml;
    /// let string = "Hello".to_string();
    /// let my_xml = Xml(string);
    /// assert_eq!(my_xml.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'r, T: DeserializeOwned> Xml<T> {
    fn from_str(s: &'r str) -> Result<Self, Error> {
        quick_xml::de::from_str(s).map(Xml)
    }

    async fn from_req_data(req: &'r Request<'_>, data: Data) -> Result<Self, Error> {
        let size_limit = req.limits().get("xml").unwrap_or(DEFAULT_LIMIT);
        let string = match data.open(size_limit).into_string().await {
            Ok(s) if s.is_complete() => s.into_inner(),
            Ok(_) => {
                let eof = io::ErrorKind::UnexpectedEof;
                return Err(
                    Error::Xml(XmlError::Io(io::Error::new(eof, "data limit exceeded")))
                );
            },
            Err(e) => return Err(Error::Xml(XmlError::Io(e))),
        };

        Self::from_str(local_cache!(req, string))
    }
}

#[rocket::async_trait]
impl<'r, T: DeserializeOwned> FromData<'r> for Xml<T> {
    type Error = Error;

    async fn from_data(req: &'r Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        match Self::from_req_data(req, data).await {
            Ok(value) => Outcome::Success(value),
            Err(Error::Xml(XmlError::Io(e))) if e.kind() == io::ErrorKind::UnexpectedEof => {
                Outcome::Failure((Status::PayloadTooLarge, Error::Xml(XmlError::Io(e))))
            },
            Err(e) => Outcome::Failure((Status::BadRequest, e)),
        }
    }
}

/// Serializes the wrapped value into XML. Returns a response with Content-Type
/// XML and a fixed-size body with the serialized value. If serialization
/// fails, an `Err` of `Status::InternalServerError` is returned.
impl<'r, T: Serialize> Responder<'r, 'static> for Xml<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let string = quick_xml::se::to_string(&self.0)
            .map_err(|e| {
                error_!("XML failed to serialize: {:?}", e);
                Status::InternalServerError
            })?;

        content::Xml(string).respond_to(req)
    }
}

#[rocket::async_trait]
impl<'v, T: DeserializeOwned + Send> form::FromFormField<'v> for Xml<T> {
    fn from_value(field: form::ValueField<'v>) -> Result<Self, form::Errors<'v>> {
        Self::from_str(field.value).map_err(|e| form::Error::custom(e).into())
    }

    async fn from_data(f: form::DataField<'v, '_>) -> Result<Self, form::Errors<'v>> {
        Self::from_req_data(f.request, f.data)
            .await
            .map_err(|e| form::Error::custom(e).into())
    }
}

impl<T> From<T> for Xml<T> {
    fn from(value: T) -> Self {
        Xml(value)
    }
}

impl<T> Deref for Xml<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Xml<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}