//! Automatic XML (de)serialization support.
//!
//! See [`Xml`] for details.
//!
//! # Enabling
//!
//! This module is only available when the `xml` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket]
//! version = "0.6.0-dev"
//! features = ["xml"]
//! ```
//!
//! # Testing
//!
//! The [`LocalRequest`] and [`LocalResponse`] types provide [`xml()`] and
//! [`into_xml()`] methods to create a request with serialized XML and
//! deserialize a response as XML, respectively.
//!
//! [`LocalRequest`]: crate::local::blocking::LocalRequest
//! [`LocalResponse`]: crate::local::blocking::LocalResponse
//! [`xml()`]: crate::local::blocking::LocalRequest::xml()
//! [`into_xml()`]: crate::local::blocking::LocalResponse::into_xml()

use std::{io, fmt, error};
use std::ops::{Deref, DerefMut};

use crate::request::{Request, local_cache};
use crate::data::{Limits, Data, FromData, Outcome};
use crate::response::{self, Responder, content};
use crate::form::prelude as form;
use crate::http::uri::fmt::{UriDisplay, FromUriParam, Query, Formatter as UriFormatter};
use crate::http::Status;

use serde::{Serialize, Deserialize};

#[doc(hidden)]
pub use quick_xml;

/// The XML guard: easily consume and return XML.
///
/// ## Sending XML
///
/// To respond with serialized XML data, return a `Xml<T>` type, where `T`
/// implements [`Serialize`] from [`serde`]. The content type of the response is
/// set to `text/xml; charset=utf-8` automatically.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::serde::{Serialize, xml::Xml};
///
/// #[derive(Serialize)]
/// #[serde(crate = "rocket::serde")]
/// struct User {
///     id: usize,
/// }
///
/// #[get("/users/<id>")]
/// fn user(id: usize) -> Xml<User> {
///     let user_from_id = User { id };
///     /* ... */
///     Xml(user_from_id)
/// }
/// ```
///
/// XML requires a root element, so `T` must serialize as a named element, such
/// as a struct or enum. Values without a name, like integers, strings,
/// sequences, and tuples, fail to serialize, resulting in a `500` response.
///
/// ## Receiving XML
///
/// `Xml` is both a data guard and a form guard.
///
/// ### Data Guard
///
/// To deserialize request body data as XML , add a `data` route argument with
/// a target type of `Xml<T>`, where `T` is some type you'd like to parse from
/// XML. `T` must implement [`serde::Deserialize`].
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket::serde::Deserialize;
/// # #[derive(Deserialize)]
/// # #[serde(crate = "rocket::serde")]
/// # struct User { id: usize }
/// use rocket::serde::xml::Xml;
///
/// #[post("/user", format = "xml", data = "<user>")]
/// fn new_user(user: Xml<User>) {
///     /* ... */
/// }
/// ```
///
/// The name of the root element is not checked: `<Other>...</Other>`
/// deserializes into a `User` just as `<User>...</User>` does.
///
/// You don't _need_ to use `format = "xml"`, but it _may_ be what you want.
/// Using `format = xml` means that any request that doesn't specify
/// "text/xml" as its `Content-Type` header value will not be routed to
/// the handler. Use `format = "*/xml"` to accept both `text/xml` and
/// `application/xml`.
///
/// ### Form Guard
///
/// `Xml<T>`, as a form guard, accepts value and data fields and parses the
/// data as a `T`. Simple use `Xml<T>`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket::serde::Deserialize;
/// # #[derive(Deserialize)]
/// # #[serde(crate = "rocket::serde")]
/// # struct Metadata { version: usize }
/// use rocket::form::{Form, FromForm};
/// use rocket::serde::xml::Xml;
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
/// ### Incoming Data Limits
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
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xml<T>(pub T);

/// Error returned by the [`Xml`] guard when XML deserialization fails.
#[derive(Debug)]
pub enum Error<'a> {
    /// An I/O error occurred while reading the incoming request data.
    Io(io::Error),

    /// The client's data was received successfully but failed to parse as valid
    /// XML or as the requested type. The `&str` value in `.0` is the raw data
    /// received from the user, while the `Error` in `.1` is the deserialization
    /// error from `quick_xml`.
    Parse(&'a str, quick_xml::DeError),
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "i/o error: {}", err),
            Self::Parse(_, err) => write!(f, "parse error: {}", err),
        }
    }
}

impl<'a> error::Error for Error<'a> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(_, err) => Some(err),
        }
    }
}

impl<T> Xml<T> {
    /// Consumes the XML wrapper and returns the wrapped item.
    ///
    /// # Example
    /// ```rust
    /// # use rocket::serde::xml::Xml;
    /// let string = "Hello".to_string();
    /// let my_xml = Xml(string);
    /// assert_eq!(my_xml.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'r, T: Deserialize<'r>> Xml<T> {
    fn from_str(s: &'r str) -> Result<Self, Error<'r>> {
        quick_xml::de::from_str(s).map(Xml).map_err(|e| Error::Parse(s, e))
    }

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Result<Self, Error<'r>> {
        let limit = req.limits().get("xml").unwrap_or(Limits::XML);
        let string = match data.open(limit).into_string().await {
            Ok(s) if s.is_complete() => s.into_inner(),
            Ok(_) => {
                let eof = io::ErrorKind::UnexpectedEof;
                return Err(Error::Io(io::Error::new(eof, "data limit exceeded")));
            },
            Err(e) => return Err(Error::Io(e)),
        };

        Self::from_str(local_cache!(req, string))
    }
}

#[crate::async_trait]
impl<'r, T: Deserialize<'r>> FromData<'r> for Xml<T> {
    type Error = Error<'r>;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        match Self::from_data(req, data).await {
            Ok(value) => Outcome::Success(value),
            Err(Error::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                Outcome::Error((Status::PayloadTooLarge, Error::Io(e)))
            },
            Err(e) => Outcome::Error((Status::BadRequest, e)),
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
                error!("XML serialize failure: {}", e);
                Status::InternalServerError
            })?;

        content::RawXml(string).respond_to(req)
    }
}

impl<T: Serialize> UriDisplay<Query> for Xml<T> {
    fn fmt(&self, f: &mut UriFormatter<'_, Query>) -> fmt::Result {
        let string = to_string(&self.0).map_err(|_| fmt::Error)?;
        f.write_value(&string)
    }
}

macro_rules! impl_from_uri_param_from_inner_type {
    ($($lt:lifetime)?, $T:ty) => (
        impl<$($lt,)? T: Serialize> FromUriParam<Query, $T> for Xml<T> {
            type Target = Xml<$T>;

            #[inline(always)]
            fn from_uri_param(param: $T) -> Self::Target {
                Xml(param)
            }
        }
    )
}

impl_from_uri_param_from_inner_type!(, T);
impl_from_uri_param_from_inner_type!('a, &'a T);
impl_from_uri_param_from_inner_type!('a, &'a mut T);

crate::http::impl_from_uri_param_identity!([Query] (T: Serialize) Xml<T>);

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

impl From<Error<'_>> for form::Error<'_> {
    fn from(e: Error<'_>) -> Self {
        match e {
            Error::Io(e) => e.into(),
            Error::Parse(_, e) => form::Error::custom(e)
        }
    }
}

#[crate::async_trait]
impl<'v, T: Deserialize<'v> + Send> form::FromFormField<'v> for Xml<T> {
    fn from_value(field: form::ValueField<'v>) -> Result<Self, form::Errors<'v>> {
        Ok(Self::from_str(field.value)?)
    }

    async fn from_data(f: form::DataField<'v, '_>) -> Result<Self, form::Errors<'v>> {
        Ok(Self::from_data(f.request, f.data).await?)
    }
}

/// Deserialize an instance of type `T` from a string of XML text.
///
/// **_Always_ use [`Xml`] to deserialize XML request data.**
///
/// # Example
///
/// ```
/// use rocket::serde::{Deserialize, xml};
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// #[serde(crate = "rocket::serde")]
/// struct Data<'r> {
///     framework: &'r str,
///     stars: usize,
/// }
///
/// let string = r#"
///     <Data>
///         <framework>Rocket</framework>
///         <stars>5</stars>
///     </Data>
/// "#;
///
/// let data: Data = xml::from_str(string).unwrap();
/// assert_eq!(data, Data { framework: "Rocket", stars: 5, });
/// ```
///
/// # Errors
///
/// This conversion can fail if the structure of the input does not match the
/// structure expected by `T`, for example if `T` is a struct type but the input
/// contains something other than a XML element. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the XML element or some number is too big to fit in the expected primitive
/// type.
#[inline(always)]
pub fn from_str<'a, T>(string: &'a str) -> Result<T, quick_xml::DeError>
    where T: Deserialize<'a>,
{
    quick_xml::de::from_str(string)
}

/// Serialize a `T` into a XML string with compact representation.
///
/// **_Always_ use [`Xml`] to serialize XML response data.**
///
/// # Example
///
/// ```
/// use rocket::serde::{Deserialize, Serialize, xml};
///
/// #[derive(Debug, PartialEq, Deserialize, Serialize)]
/// #[serde(crate = "rocket::serde")]
/// struct Data<'r> {
///     framework: &'r str,
///     stars: usize,
/// }
///
/// let data = Data {
///     framework: "Rocket",
///     stars: 5,
/// };
///
/// let string = xml::to_string(&data).unwrap();
/// let data: Data = xml::from_str(&string).unwrap();
/// assert_eq!(data, Data { framework: "Rocket", stars: 5, });
/// ```
///
/// # Errors
///
/// Serialization fails if `T`'s `Serialize` implementation fails.
#[inline(always)]
pub fn to_string<T>(value: &T) -> Result<String, quick_xml::SeError>
    where T: Serialize
{
    quick_xml::se::to_string(value)
}
