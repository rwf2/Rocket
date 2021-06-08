//! Automatic MessagePack (de)serialization support.
//!
//! See [`MsgPack`](crate::serde::msgpack::MsgPack) for further details.
//!
//! # Enabling
//!
//! This module is only available when the `json` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket]
//! version = "0.5.0-dev"
//! features = ["msgpack"]
//! ```
//!
//! # Testing
//!
//! The [`LocalRequest`] and [`LocalResponse`] types provide [`msgpack()`] and
//! [`into_msgpack()`] methods to create a request with serialized MessagePack
//! and deserialize a response as MessagePack, respectively.
//!
//! [`LocalRequest`]: crate::local::blocking::LocalRequest
//! [`LocalResponse`]: crate::local::blocking::LocalResponse
//! [`msgpack()`]: crate::local::blocking::LocalRequest::msgpack()
//! [`into_msgpack()`]: crate::local::blocking::LocalResponse::into_msgpack()

use std::io;
use std::ops::{Deref, DerefMut};

use crate::request::{Request, local_cache};
use crate::data::{Limits, Data, FromData, Outcome};
use crate::response::{self, Responder, content};
use crate::http::Status;
use crate::form::prelude as form;

use serde::{Serialize, Deserialize};

#[doc(inline)]
pub use rmp_serde::decode::Error;

/// The MessagePack guard: easily consume and return MessagePack.
///
/// ## Receiving MessagePack
///
/// `MsgPack` is both a data guard and a form guard.
///
/// ### Data Guard
///
/// To parse request body data as MessagePack , add a `data` route argument with
/// a target type of `MsgPack<T>`, where `T` is some type you'd like to parse
/// from JSON. `T` must implement [`serde::Deserialize`].
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type User = usize;
/// use rocket::serde::msgpack::MsgPack;
///
/// #[post("/users", format = "msgpack", data = "<user>")]
/// fn new_user(user: MsgPack<User>) {
///     /* ... */
/// }
/// ```
///
/// You don't _need_ to use `format = "msgpack"`, but it _may_ be what you want.
/// Using `format = msgpack` means that any request that doesn't specify
/// "application/msgpack" as its first `Content-Type:` header parameter will not
/// be routed to this handler.
///
/// ### Form Guard
///
/// `MsgPack<T>`, as a form guard, accepts value and data fields and parses the
/// data as a `T`. Simple use `MsgPack<T>`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type Metadata = usize;
/// use rocket::form::{Form, FromForm};
/// use rocket::serde::msgpack::MsgPack;
///
/// #[derive(FromForm)]
/// struct User<'r> {
///     name: &'r str,
///     metadata: MsgPack<Metadata>
/// }
///
/// #[post("/users", data = "<form>")]
/// fn new_user(form: Form<User<'_>>) {
///     /* ... */
/// }
/// ```
///
/// ### Incoming Data Limits
///
/// The default size limit for incoming MessagePack data is 1MiB. Setting a
/// limit protects your application from denial of service (DOS) attacks and
/// from resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.msgpack` configuration parameter. For
/// instance, to increase the MessagePack limit to 5MiB for all environments,
/// you may add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// msgpack = 5242880
/// ```
///
/// ## Sending MessagePack
///
/// If you're responding with MessagePack data, return a `MsgPack<T>` type,
/// where `T` implements [`Serialize`] from [`serde`]. The content type of the
/// response is set to `application/msgpack` automatically.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type User = usize;
/// use rocket::serde::msgpack::MsgPack;
///
/// #[get("/users/<id>")]
/// fn user(id: usize) -> MsgPack<User> {
///     let user_from_id = User::from(id);
///     /* ... */
///     MsgPack(user_from_id)
/// }
/// ```
#[derive(Debug)]
pub struct MsgPack<T>(pub T);

impl<T> MsgPack<T> {
    /// Consumes the `MsgPack` wrapper and returns the wrapped item.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::serde::msgpack::MsgPack;
    /// let string = "Hello".to_string();
    /// let my_msgpack = MsgPack(string);
    /// assert_eq!(my_msgpack.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'r, T: Deserialize<'r>> MsgPack<T> {
    fn from_bytes(buf: &'r [u8]) -> Result<Self, Error> {
        rmp_serde::from_slice(buf).map(MsgPack)
    }

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Result<Self, Error> {
        let limit = req.limits().get("msgpack").unwrap_or(Limits::MESSAGE_PACK);
        let bytes = match data.open(limit).into_bytes().await {
            Ok(buf) if buf.is_complete() => buf.into_inner(),
            Ok(_) => {
                let eof = io::ErrorKind::UnexpectedEof;
                return Err(Error::InvalidDataRead(io::Error::new(eof, "data limit exceeded")));
            },
            Err(e) => return Err(Error::InvalidDataRead(e)),
        };

        Self::from_bytes(local_cache!(req, bytes))
    }
}

#[crate::async_trait]
impl<'r, T: Deserialize<'r>> FromData<'r> for MsgPack<T> {
    type Error = Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        match Self::from_data(req, data).await {
            Ok(value) => Outcome::Success(value),
            Err(Error::InvalidDataRead(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                Outcome::Failure((Status::PayloadTooLarge, Error::InvalidDataRead(e)))
            },
            | Err(e@Error::TypeMismatch(_))
            | Err(e@Error::OutOfRange)
            | Err(e@Error::LengthMismatch(_))
            => {
                Outcome::Failure((Status::UnprocessableEntity, e))
            },
            Err(e) => Outcome::Failure((Status::BadRequest, e)),
        }
    }
}

/// Serializes the wrapped value into MessagePack. Returns a response with
/// Content-Type `MsgPack` and a fixed-size body with the serialization. If
/// serialization fails, an `Err` of `Status::InternalServerError` is returned.
impl<'r, T: Serialize> Responder<'r, 'static> for MsgPack<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let buf = rmp_serde::to_vec(&self.0)
            .map_err(|e| {
                error_!("MsgPack failed to serialize: {:?}", e);
                Status::InternalServerError
            })?;

        content::MsgPack(buf).respond_to(req)
    }
}

#[crate::async_trait]
impl<'v, T: Deserialize<'v> + Send> form::FromFormField<'v> for MsgPack<T> {
    async fn from_data(f: form::DataField<'v, '_>) -> Result<Self, form::Errors<'v>> {
        Self::from_data(f.request, f.data).await.map_err(|e| {
            match e {
                Error::InvalidMarkerRead(e) | Error::InvalidDataRead(e) => e.into(),
                Error::Utf8Error(e) => e.into(),
                _ => form::Error::custom(e).into(),
            }
        })
    }
}

impl<T> From<T> for MsgPack<T> {
    fn from(value: T) -> Self {
        MsgPack(value)
    }
}

impl<T> Deref for MsgPack<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for MsgPack<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// Deserialize an instance of type `T` from MessagePack encoded bytes.
///
/// Deserialization is performed in a zero-copy manner whenever possible.
///
/// # Example
///
/// ```
/// use rocket::serde::{Deserialize, msgpack};
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// #[serde(crate = "rocket::serde")]
/// struct Data<'r> {
///     framework: &'r str,
///     stars: usize,
/// }
///
/// let bytes = &[
///     130, 169, 102, 114, 97, 109, 101, 119, 111, 114, 107, 166, 82, 111,
///     99, 107, 101, 116, 165, 115, 116, 97, 114, 115, 5
/// ];
///
/// let data: Data = msgpack::from_slice(bytes).unwrap();
/// assert_eq!(data, Data { framework: "Rocket", stars: 5, });
/// ```
///
/// # Errors
///
/// Deserialization fails if `v` does not represent a valid MessagePack encoding
/// of any instance of `T` or if `T`'s `Deserialize` implementation fails
/// otherwise.
#[inline(always)]
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T, Error>
    where T: Deserialize<'a>,
{
    rmp_serde::from_read_ref(v)
}
