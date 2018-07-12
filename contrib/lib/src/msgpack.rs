extern crate rmp_serde;

use std::ops::{Deref, DerefMut};
use std::io::{Cursor, Read};

use rocket::outcome::{Outcome, IntoOutcome};
use rocket::request::Request;
use rocket::data::{self, Data, FromData};
use rocket::response::{self, Responder, Response};
use rocket::http::{ContentType, Status};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub use self::rmp_serde::decode::Error as MsgPackError;

/// The `MsgPack` type: implements `FromData` and `Responder`, allowing you to
/// easily consume and respond with MessagePack data.
///
/// ## Receiving MessagePack
///
/// If you're receiving MessagePack data, simply add a `data` parameter to your
/// route arguments and ensure the type of the parameter is a `MsgPack<T>`,
/// where `T` is some type you'd like to parse from MessagePack. `T` must
/// implement `Deserialize` or `DeserializeOwned` from
/// [Serde](https://github.com/serde-rs/serde). The data is parsed from the HTTP
/// request body.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// # fn main() {  }
/// #
/// use rocket_contrib::MsgPack;
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
/// be routed to this handler. By default, Rocket will accept a Content-Type of
/// any of the following for MessagePack data: `application/msgpack`,
/// `application/x-msgpack`, `bin/msgpack`, or `bin/x-msgpack`.
///
/// ## Sending MessagePack
///
/// If you're responding with MessagePack data, return a `MsgPack<T>` type,
/// where `T` implements `Serialize` from
/// [serde](https://github.com/serde-rs/serde). The content type of the response
/// is set to `application/msgpack` automatically.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// # fn main() {  }
/// #
/// use rocket_contrib::MsgPack;
///
/// #[get("/users/<id>")]
/// fn user(id: usize) -> MsgPack<User> {
///     let user_from_id = User::from(id);
///     /* ... */
///     MsgPack(user_from_id)
/// }
/// ```
///
/// ## Incoming Data Limits
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
#[derive(Debug)]
pub struct MsgPack<T>(pub T);

impl<T> MsgPack<T> {
    /// Consumes the `MsgPack` wrapper and returns the wrapped item.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket_contrib::MsgPack;
    /// let string = "Hello".to_string();
    /// let my_msgpack = MsgPack(string);
    /// assert_eq!(my_msgpack.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Default limit for MessagePack is 1MB.
const LIMIT: u64 = 1 << 20;

/// Accepted content types are: `application/msgpack`, `application/x-msgpack`,
/// `bin/msgpack`, and `bin/x-msgpack`.
#[inline(always)]
fn is_msgpack_content_type(ct: &ContentType) -> bool {
    (ct.top() == "application" || ct.top() == "bin")
        && (ct.sub() == "msgpack" || ct.sub() == "x-msgpack")
}

impl<T: DeserializeOwned> FromData for MsgPack<T> {
    type Error = MsgPackError;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        if !request.content_type().map_or(false, |ct| is_msgpack_content_type(&ct)) {
            error_!("Content-Type is not MessagePack.");
            return Outcome::Forward(data);
        }

        let mut buf = Vec::new();
        let size_limit = request.limits().get("msgpack").unwrap_or(LIMIT);
        if let Err(e) = data.open().take(size_limit).read_to_end(&mut buf) {
            let e = MsgPackError::InvalidDataRead(e);
            error_!("Couldn't read request data: {:?}", e);
            return Outcome::Failure((Status::BadRequest, e));
        };

        rmp_serde::from_slice(&buf).map(|val| MsgPack(val))
            .map_err(|e| { error_!("Couldn't parse MessagePack body: {:?}", e); e })
            .into_outcome(Status::BadRequest)
    }
}

/// Serializes the wrapped value into MessagePack. Returns a response with
/// Content-Type `MsgPack` and a fixed-size body with the serialization. If
/// serialization fails, an `Err` of `Status::InternalServerError` is returned.
impl<T: Serialize> Responder<'static> for MsgPack<T> {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        rmp_serde::to_vec(&self.0).map_err(|e| {
            error_!("MsgPack failed to serialize: {:?}", e);
            Status::InternalServerError
        }).and_then(|buf| {
            Response::build()
                .sized_body(Cursor::new(buf))
                .ok()
        })
    }
}

impl<T> Deref for MsgPack<T> {
    type Target = T;

    #[inline(always)]
    fn deref<'a>(&'a self) -> &'a T {
        &self.0
    }
}

impl<T> DerefMut for MsgPack<T> {
    #[inline(always)]
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.0
    }
}
