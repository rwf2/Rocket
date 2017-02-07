use outcome::{self, IntoOutcome};
use outcome::Outcome::*;
use http::Status;
use request::Request;
use data::Data;

/// Type alias for the `Outcome` of a `FromData` conversion.
pub type Outcome<S, E> = outcome::Outcome<S, (Status, E), Data>;

impl<'a, S, E> IntoOutcome<S, (Status, E), Data> for Result<S, E> {
    #[inline]
    fn into_outcome(self) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(err) => Failure((Status::InternalServerError, err))
        }
    }
}

/// Trait used to derive an object from incoming request data.
///
/// Types that implement this trait can be used as a target for the `data =
/// "<param>"` route parmater, as illustrated below:
///
/// ```rust,ignore
/// #[post("/submit", data = "<var>")]
/// fn submit(var: T) -> ... { ... }
/// ```
///
/// In this example, `T` can be any type that implements `FromData`.
///
/// # Outcomes
///
/// The returned [Outcome](/rocket/outcome/index.html) of a `from_data` call
/// determines how the incoming request will be processed.
///
/// * **Success**(S)
///
///   If the `Outcome` is `Success`, then the `Success` value will be used as
///   the value for the data parameter.  As long as all other parsed types
///   succeed, the request will be handled by the requesting handler.
///
/// * **Failure**(Status, E)
///
///   If the `Outcome` is `Failure`, the request will fail with the given status
///   code and error. The designated error
///   [Catcher](/rocket/struct.Catcher.html) will be used to respond to the
///   request. Note that users can request types of `Result<S, E>` and
///   `Option<S>` to catch `Failure`s and retrieve the error value.
///
/// * **Forward**(Data)
///
///   If the `Outcome` is `Forward`, the request will be forwarded to the next
///   matching request. This requires that no data has been read from the `Data`
///   parameter. Note that users can request an `Option<S>` to catch `Forward`s.
///
/// # Example
///
/// Say that you have a custom type, `Person`:
///
/// ```rust
/// # #[allow(dead_code)]
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// `Person` has a custom serialization format, so the built-in `JSON` type
/// doesn't suffice. The format is `<name>:<age>` with `Content-Type:
/// application/x-person`. You'd like to use `Person` as a `FromData` type so
/// that you can retrieve it directly from a client's request body:
///
/// ```rust,ignore
/// #[post("/person", data = "<person>")]
/// fn person(person: Person) -> &'static str {
///     "Saved the new person to the database!"
/// }
/// ```
///
/// A `FromData` implementation allowing this looks like:
///
/// ```rust
/// # #![allow(unused_attributes)]
/// # #![allow(unused_variables)]
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person { name: String, age: u16 }
/// #
/// use std::io::Read;
/// use rocket::{Request, Data, Outcome};
/// use rocket::data::{self, FromData};
/// use rocket::http::{Status, ContentType};
/// use rocket::Outcome::*;
///
/// impl FromData for Person {
///     type Error = String;
///
///     fn from_data(req: &Request, data: Data) -> data::Outcome<Self, String> {
///         // Ensure the content type is correct before opening the data.
///         let person_ct = ContentType::new("application", "x-person");
///         if req.content_type() != Some(person_ct) {
///             return Outcome::Forward(data);
///         }
///
///         // Read the data into a String.
///         let mut string = String::new();
///         if let Err(e) = data.open().read_to_string(&mut string) {
///             return Failure((Status::InternalServerError, format!("{:?}", e)));
///         }
///
///         // Split the string into two pieces at ':'.
///         let (name, age) = match string.find(':') {
///             Some(i) => (&string[..i], &string[(i + 1)..]),
///             None => return Failure((Status::BadRequest, "Missing ':'.".into()))
///         };
///
///         // Parse the age.
///         let age: u16 = match age.parse() {
///             Ok(age) => age,
///             Err(_) => return Failure((Status::BadRequest, "Bad age.".into()))
///         };
///
///         // Return successfully.
///         Success(Person {
///             name: name.into(),
///             age: age
///         })
///     }
/// }
/// #
/// # #[post("/person", data = "<person>")]
/// # fn person(person: Person) {  }
/// # #[post("/person", data = "<person>")]
/// # fn person2(person: Result<Person, String>) {  }
/// # fn main() {  }
/// ```
pub trait FromData: Sized {
    /// The associated error to be returned when parsing fails.
    type Error;

    /// Parses an instance of `Self` from the incoming request body data.
    ///
    /// If the parse is successful, an outcome of `Success` is returned. If the
    /// data does not correspond to the type of `Self`, `Forward` is returned.
    /// If parsing fails, `Failure` is returned.
    fn from_data(request: &Request, data: Data) -> Outcome<Self, Self::Error>;
}

/// The identity implementation of `FromData`. Always returns `Success`.
impl FromData for Data {
    type Error = ();
    fn from_data(_: &Request, data: Data) -> Outcome<Self, Self::Error> {
        Success(data)
    }
}

impl<T: FromData> FromData for Result<T, T::Error> {
    type Error = ();

    fn from_data(request: &Request, data: Data) -> Outcome<Self, Self::Error> {
        match T::from_data(request, data) {
            Success(val) => Success(Ok(val)),
            Failure((_, val)) => Success(Err(val)),
            Forward(data) => Forward(data),
        }
    }
}

impl<T: FromData> FromData for Option<T> {
    type Error = ();

    fn from_data(request: &Request, data: Data) -> Outcome<Self, Self::Error> {
        match T::from_data(request, data) {
            Success(val) => Success(Some(val)),
            Failure(_) | Forward(_) => Success(None),
        }
    }
}
