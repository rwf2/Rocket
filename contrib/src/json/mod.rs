extern crate serde;
extern crate serde_json;

use std::ops::{Deref, DerefMut};
use std::io::Read;

use rocket::outcome::Outcome;
use rocket::request::Request;
use rocket::data::{self, Data, FromData};
use rocket::response::{self, Responder, content};
use rocket::http::Status;

use self::serde::{Serialize, Deserialize};

pub use self::serde_json::error::Error as SerdeError;

/// The JSON type, which implements `FromData` and `Responder`. This type allows
/// you to trivially consume and respond with JSON in your Rocket application.
///
/// If you're receiving JSON data, simple add a `data` parameter to your route
/// arguments and ensure the type o the parameter is a `JSON<T>`, where `T` is
/// some type you'd like to parse from JSON. `T` must implement `Deserialize`
/// from [Serde](https://github.com/serde-rs/json). The data is parsed from the
/// HTTP request body.
///
/// ```rust,ignore
/// #[post("/users/", format = "application/json", data = "<user>")]
/// fn new_user(user: JSON<User>) {
///     ...
/// }
/// ```
/// You don't _need_ to use `format = "application/json"`, but it _may_ be what
/// you want. Using `format = application/json` means that any request that
/// doesn't specify "application/json" as its first `Content-Type:` header
/// parameter will not be routed to this handler.
///
/// If you're responding with JSON data, return a `JSON<T>` type, where `T`
/// implements `Serialize` from [Serde](https://github.com/serde-rs/json). The
/// content type of the response is set to `application/json` automatically.
///
/// ```rust,ignore
/// #[get("/users/<id>")]
/// fn user(id: usize) -> JSON<User> {
///     let user_from_id = User::from(id);
///     ...
///     JSON(user_from_id)
/// }
/// ```
///
#[derive(Debug)]
pub struct JSON<T>(pub T);

impl<T> JSON<T> {
    /// Consumes the JSON wrapper and returns the wrapped item.
    ///
    /// # Example
    /// ```rust
    /// # use rocket_contrib::JSON;
    /// let string = "Hello".to_string();
    /// let my_json = JSON(string);
    /// assert_eq!(my_json.unwrap(), "Hello".to_string());
    /// ```
    pub fn unwrap(self) -> T {
        self.0
    }
}

/// Maximum size of JSON is 1MB.
/// TODO: Determine this size from some configuration parameter.
const MAX_SIZE: u64 = 1048576;

impl<T: Deserialize> FromData for JSON<T> {
    type Error = SerdeError;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, SerdeError> {
        if !request.content_type().is_json() {
            error_!("Content-Type is not JSON.");
            return Outcome::Forward(data);
        }

        let reader = data.open().take(MAX_SIZE);
        match serde_json::from_reader(reader).map(|val| JSON(val)) {
            Ok(value) => Outcome::Success(value),
            Err(e) => {
                error_!("Couldn't parse JSON body: {:?}", e);
                Outcome::Failure((Status::BadRequest, e))
            }
        }
    }
}

// Serializes the wrapped value into JSON. Returns a response with Content-Type
// JSON and a fixed-size body with the serialization. If serialization fails, an
// `Err` of `Status::InternalServerError` is returned.
impl<T: Serialize> Responder<'static> for JSON<T> {
    fn respond(self) -> response::Result<'static> {
        serde_json::to_string(&self.0).map(|string| {
            content::JSON(string).respond().unwrap()
        }).map_err(|e| {
            error_!("JSON failed to serialize: {:?}", e);
            Status::InternalServerError
        })
    }
}

impl<T> Deref for JSON<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a T {
        &self.0
    }
}

impl<T> DerefMut for JSON<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.0
    }
}

/// A nice little macro to create simple HashMaps. Really convenient for
/// returning ad-hoc JSON messages.
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate rocket_contrib;
/// use std::collections::HashMap;
/// # fn main() {
/// let map: HashMap<&str, usize> = map! {
///     "status" => 0,
///     "count" => 100
/// };
///
/// assert_eq!(map.len(), 2);
/// assert_eq!(map.get("status"), Some(&0));
/// assert_eq!(map.get("count"), Some(&100));
/// # }
/// ```
#[macro_export]
macro_rules! map {
    ($($key:expr => $value:expr),+) => ({
        use std::collections::HashMap;
        let mut map = HashMap::new();
        $(map.insert($key, $value);)+
        map
    });

    ($($key:expr => $value:expr),+,) => {
        map!($($key => $value),+)
    };
}
