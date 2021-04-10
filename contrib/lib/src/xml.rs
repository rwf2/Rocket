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

use rocket::data::{ByteUnit, FromData, Outcome};
use rocket::response::{self, Responder, content};
use rocket::request::Request;
use rocket::http::Status;
use rocket::{Data, form};
pub use quick_xml::DeError as Error;
use quick_xml::Error as XmlError;
use std::io;

use serde::{Serialize, Serializer};
use serde::de::{Deserialize, DeserializeOwned, Deserializer};
use std::ops::{Deref, DerefMut};

// TODO Struct docs
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
        // TODO: Should the error handling here be improved? Different Error type?
        quick_xml::de::from_str(s).map(Xml)
    }

    async fn from_req_data(req: &'r Request<'_>, data: Data) -> Result<Self, Error> {
        let size_limit = req.limits().get("xml").unwrap_or(DEFAULT_LIMIT);
        let string = match data.open(size_limit).into_string().await {
            Ok(s) if s.is_complete() => s.into_inner(),
            Ok(_) => {
                let eof = io::ErrorKind::UnexpectedEof;
                return Err(Error::Xml(XmlError::Io(io::Error::new(eof, "data limit exceeded"))));
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