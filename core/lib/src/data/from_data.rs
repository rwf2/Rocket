use std::io::{self, Read};

use outcome::{self, IntoOutcome};
use outcome::Outcome::*;
use http::Status;
use request::Request;
use data::Data;

/// Type alias for the `Outcome` of a `FromData` conversion.
pub type Outcome<S, E> = outcome::Outcome<S, (Status, E), Data>;

impl<'a, S, E> IntoOutcome<S, (Status, E), Data> for Result<S, E> {
    type Failure = Status;
    type Forward = Data;

    #[inline]
    fn into_outcome(self, status: Status) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(err) => Failure((status, err))
        }
    }

    #[inline]
    fn or_forward(self, data: Data) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(_) => Forward(data)
        }
    }
}

/// Trait implemented by data guards to derive a value from request body data.
///
/// # Data Guards
///
/// A data guard is a [request guard] that operates on a request's body data.
/// Data guards validate, parse, and optionally convert request body data.
/// Validation and parsing/conversion is implemented through `FromData`. In
/// other words, every type that implements `FromData` is a data guard.
///
/// [request guard]: /rocket/request/trait.FromRequest.html
///
/// Data guards are used as the target of the `data` route attribute parameter.
/// A handler can have at most one data guard.
///
/// ## Example
///
/// In the example below, `var` is used as the argument name for the data guard
/// type `T`. When the `submit` route matches, Rocket will call the `FromData`
/// implementation for the type `T`. The handler will only be called if the guard
/// returns successfully.
///
/// ```rust,ignore
/// #[post("/submit", data = "<var>")]
/// fn submit(var: T) -> ... { ... }
/// ```
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
/// # Provided Implementations
///
/// Rocket implements `FromData` for several built-in types. Their behavior is
/// documented here.
///
///   * **Data**
///
///     The identity implementation; simply returns `Data` directly.
///
///     _This implementation always returns successfully._
///
///   * **Option&lt;T>** _where_ **T: FromData**
///
///     The type `T` is derived from the incoming data using `T`'s `FromData`
///     implementation. If the derivation is a `Success`, the derived value is
///     returned in `Some`. Otherwise, a `None` is returned.
///
///     _This implementation always returns successfully._
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromData**
///
///     The type `T` is derived from the incoming data using `T`'s `FromData`
///     implementation. If derivation is a `Success`, the value is returned in
///     `Ok`. If the derivation is a `Failure`, the error value is returned in
///     `Err`. If the derivation is a `Forward`, the request is forwarded.
///
///   * **String**
///
///     Reads the entire request body into a `String`. If reading fails, returns
///     a `Failure` with the corresponding `io::Error`.
///
///     **WARNING:** Do **not** use this implementation for anything _but_
///     debugging. This is because the implementation reads the entire body into
///     memory; since the user controls the size of the body, this is an obvious
///     vector for a denial of service attack.
///
///   * **Vec&lt;u8>**
///
///     Reads the entire request body into a `Vec<u8>`. If reading fails,
///     returns a `Failure` with the corresponding `io::Error`.
///
///     **WARNING:** Do **not** use this implementation for anything _but_
///     debugging. This is because the implementation reads the entire body into
///     memory; since the user controls the size of the body, this is an obvious
///     vector for a denial of service attack.
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
/// `Person` has a custom serialization format, so the built-in `Json` type
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
/// # #![feature(plugin, decl_macro)]
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
///         if req.content_type() != Some(&person_ct) {
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
///             None => return Failure((Status::UnprocessableEntity, "':'".into()))
///         };
///
///         // Parse the age.
///         let age: u16 = match age.parse() {
///             Ok(age) => age,
///             Err(_) => return Failure((Status::UnprocessableEntity, "Age".into()))
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
    /// The associated error to be returned when the guard fails.
    type Error;

    /// Validates, parses, and converts an instance of `Self` from the incoming
    /// request body data.
    ///
    /// If validation and parsing succeeds, an outcome of `Success` is returned.
    /// If the data is not appropriate given the type of `Self`, `Forward` is
    /// returned. If parsing fails, `Failure` is returned.
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

impl FromData for String {
    type Error = io::Error;

    fn from_data(_: &Request, data: Data) -> Outcome<Self, Self::Error> {
        let mut string = String::new();
        match data.open().read_to_string(&mut string) {
            Ok(_) => Success(string),
            Err(e) => Failure((Status::BadRequest, e))
        }
    }
}

impl FromData for Vec<u8> {
    type Error = io::Error;

    fn from_data(_: &Request, data: Data) -> Outcome<Self, Self::Error> {
        let mut bytes = Vec::new();
        match data.open().read_to_end(&mut bytes) {
            Ok(_) => Success(bytes),
            Err(e) => Failure((Status::BadRequest, e))
        }
    }
}
