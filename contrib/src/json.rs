use std::ops::{Deref, DerefMut};
use std::io::Read;

use rocket::outcome::{Outcome, IntoOutcome};
use rocket::request::Request;
use rocket::data::{self, Data, FromData};
use rocket::response::{self, Responder, content};
use rocket::http::Status;

use serde::Serialize;
use serde::de::DeserializeOwned;

use serde_json;

pub use serde_json::error::Error as SerdeError;

/// The JSON type: implements `FromData` and `Responder`, allowing you to easily
/// consume and respond with JSON.
///
/// ## Receiving JSON
///
/// If you're receiving JSON data, simply add a `data` parameter to your route
/// arguments and ensure the type of the parameter is a `Json<T>`, where `T` is
/// some type you'd like to parse from JSON. `T` must implement `Deserialize` or
/// `DeserializeOwned` from [Serde](https://github.com/serde-rs/json). The data
/// is parsed from the HTTP request body.
///
/// ```rust,ignore
/// #[post("/users/", format = "application/json", data = "<user>")]
/// fn new_user(user: Json<User>) {
///     ...
/// }
/// ```
///
/// You don't _need_ to use `format = "application/json"`, but it _may_ be what
/// you want. Using `format = application/json` means that any request that
/// doesn't specify "application/json" as its `Content-Type` header value will
/// not be routed to the handler.
///
/// ## Sending JSON
///
/// If you're responding with JSON data, return a `Json<T>` type, where `T`
/// implements `Serialize` from [Serde](https://github.com/serde-rs/json). The
/// content type of the response is set to `application/json` automatically.
///
/// ```rust,ignore
/// #[get("/users/<id>")]
/// fn user(id: usize) -> Json<User> {
///     let user_from_id = User::from(id);
///     ...
///     Json(user_from_id)
/// }
/// ```
///
/// ## Incoming Data Limits
///
/// The default size limit for incoming JSON data is 1MiB. Setting a limit
/// protects your application from denial of service (DOS) attacks and from
/// resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.json` configuration parameter. For
/// instance, to increase the JSON limit to 5MiB for all environments, you may
/// add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// json = 5242880
/// ```
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Consumes the JSON wrapper and returns the wrapped item.
    ///
    /// # Example
    /// ```rust
    /// # use rocket_contrib::Json;
    /// let string = "Hello".to_string();
    /// let my_json = Json(string);
    /// assert_eq!(my_json.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Default limit for JSON is 1MB.
const LIMIT: u64 = 1 << 20;

impl<T: DeserializeOwned> FromData for Json<T> {
    type Error = SerdeError;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, SerdeError> {
        if !request.content_type().map_or(false, |ct| ct.is_json()) {
            error_!("Content-Type is not JSON.");
            return Outcome::Forward(data);
        }

        let size_limit = request.limits().get("json").unwrap_or(LIMIT);
        serde_json::from_reader(data.open().take(size_limit))
            .map(|val| Json(val))
            .map_err(|e| { error_!("Couldn't parse JSON body: {:?}", e); e })
            .into_outcome(Status::BadRequest)
    }
}

/// Serializes the wrapped value into JSON. Returns a response with Content-Type
/// JSON and a fixed-size body with the serialized value. If serialization
/// fails, an `Err` of `Status::InternalServerError` is returned.
impl<T: Serialize> Responder<'static> for Json<T> {
    fn respond_to(self, req: &Request) -> response::Result<'static> {
        serde_json::to_string(&self.0).map(|string| {
            content::Json(string).respond_to(req).unwrap()
        }).map_err(|e| {
            error_!("JSON failed to serialize: {:?}", e);
            Status::InternalServerError
        })
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    #[inline(always)]
    fn deref<'a>(&'a self) -> &'a T {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    #[inline(always)]
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.0
    }
}

/// An arbitrary JSON value.
///
/// This structure wraps `serde`'s [`Value`] type. Importantly, unlike `Value`,
/// this type implements [`Responder`], allowing a value of this type to be
/// returned directly from a handler.
///
/// [`Value`]: https://docs.rs/serde_json/1.0.2/serde_json/value/enum.Value.html
/// [`Responder`]: /rocket/response/trait.Responder.html
///
/// # `Responder`
///
/// The `Responder` implementation for `JsonValue` serializes the represented
/// value into a JSON string and sets the string as the body of a fixed-sized
/// response with a `Content-Type` of `application/json`.
///
/// # Usage
///
/// A value of this type is constructed via the
/// [`json!`](/rocket_contrib/macro.json.html) macro. The macro and this type
/// are typically used to construct JSON values in an ad-hoc fashion during
/// request handling. This looks something like:
///
/// ```rust,ignore
/// use rocket_contrib::JsonValue;
///
/// #[get("/item")]
/// fn get_item() -> JsonValue {
///     json!({
///         "id": 83,
///         "values": [1, 2, 3, 4]
///     })
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonValue(pub serde_json::Value);

impl JsonValue {
    #[inline(always)]
    fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl Deref for JsonValue {
    type Target = serde_json::Value;

    #[inline(always)]
    fn deref<'a>(&'a self) -> &'a Self::Target {
        &self.0
    }
}

impl DerefMut for JsonValue {
    #[inline(always)]
    fn deref_mut<'a>(&'a mut self) -> &'a mut Self::Target {
        &mut self.0
    }
}

impl Into<serde_json::Value> for JsonValue {
    #[inline(always)]
    fn into(self) -> serde_json::Value {
        self.into_inner()
    }
}

impl From<serde_json::Value> for JsonValue {
    #[inline(always)]
    fn from(value: serde_json::Value) -> JsonValue {
        JsonValue(value)
    }
}

/// Serializes the value into JSON. Returns a response with Content-Type JSON
/// and a fixed-size body with the serialized value.
impl<'a> Responder<'a> for JsonValue {
    #[inline]
    fn respond_to(self, req: &Request) -> response::Result<'a> {
        content::Json(self.0.to_string()).respond_to(req)
    }
}

/// A macro to create ad-hoc JSON serializable values using JSON syntax.
///
/// # Usage
///
/// To import the macro, add the `#[macro_use]` attribute to the `extern crate
/// rocket_contrib` invocation:
///
/// ```rust,ignore
/// #[macro_use] extern crate rocket_contrib;
/// ```
///
/// The return type of a `json!` invocation is
/// [`JsonValue`](/rocket_contrib/struct.JsonValue.html). A value created with
/// this macro can be returned from a handler as follows:
///
/// ```rust,ignore
/// use rocket_contrib::JsonValue;
///
/// #[get("/json")]
/// fn get_json() -> JsonValue {
///     json!({
///         "key": "value",
///         "array": [1, 2, 3, 4]
///     })
/// }
/// ```
///
/// The `Responder` implementation for `JsonValue` serializes the value into a
/// JSON string and sets it as the body of the response with a `Content-Type` of
/// `application/json`.
///
/// # Examples
///
/// Create a simple JSON object with two keys: `"username"` and `"id"`:
///
/// ```rust
/// # #![allow(unused_variables)]
/// # #[macro_use] extern crate rocket_contrib;
/// # fn main() {
/// let value = json!({
///     "username": "mjordan",
///     "id": 23
/// });
/// # }
/// ```
///
/// Create a more complex object with a nested object and array:
///
/// ```rust
/// # #![allow(unused_variables)]
/// # #[macro_use] extern crate rocket_contrib;
/// # fn main() {
/// let value = json!({
///     "code": 200,
///     "success": true,
///     "payload": {
///         "features": ["serde", "json"],
///         "ids": [12, 121],
///     },
/// });
/// # }
/// ```
///
/// Variables or expressions can be interpolated into the JSON literal. Any type
/// interpolated into an array element or object value must implement Serde's
/// `Serialize` trait, while any type interpolated into a object key must
/// implement `Into<String>`.
///
/// ```rust
/// # #![allow(unused_variables)]
/// # #[macro_use] extern crate rocket_contrib;
/// # fn main() {
/// let code = 200;
/// let features = vec!["serde", "json"];
///
/// let value = json!({
///    "code": code,
///    "success": code == 200,
///    "payload": {
///        features[0]: features[1]
///    }
/// });
/// # }
/// ```
///
/// Trailing commas are allowed inside both arrays and objects.
///
/// ```rust
/// # #![allow(unused_variables)]
/// # #[macro_use] extern crate rocket_contrib;
/// # fn main() {
/// let value = json!([
///     "notice",
///     "the",
///     "trailing",
///     "comma -->",
/// ]);
/// # }
/// ```
#[macro_export]
macro_rules! json {
    ($($json:tt)+) => {
        $crate::JsonValue(json_internal!($($json)+))
    };
}
