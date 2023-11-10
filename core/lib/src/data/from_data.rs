use crate::http::{RawStr, Status};
use crate::request::{Request, local_cache};
use crate::data::{Data, Limits};
use crate::outcome::{self, IntoOutcome, try_outcome, Outcome::*};

/// Type alias for the `Outcome` of [`FromData`].
///
/// [`FromData`]: crate::data::FromData
pub type Outcome<'r, T, E = <T as FromData<'r>>::Error>
    = outcome::Outcome<T, (Status, E), (Data<'r>, Status)>;

/// Trait implemented by data guards to derive a value from request body data.
///
/// # Data Guards
///
/// A data guard is a guard that operates on a request's body data. Data guards
/// validate and parse request body data via implementations of `FromData`. In
/// other words, a type is a data guard _iff_ it implements `FromData`.
///
/// Data guards are the target of the `data` route attribute parameter:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type DataGuard = String;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: DataGuard) { /* ... */ }
/// ```
///
/// A route can have at most one data guard. Above, `var` is used as the
/// argument name for the data guard type `DataGuard`. When the `submit` route
/// matches, Rocket will call the `FromData` implementation for the type `T`.
/// The handler will only be called if the guard returns successfully.
///
/// ## Build-In Guards
///
/// Rocket provides implementations for `FromData` for many types. Their
/// behavior is documented here:
///
///   * `Data`: Returns the untouched `Data`.
///
///     - **Fails:** Never.
///
///     - **Succeeds:** Always.
///
///     - **Forwards:** Never.
///
///   * Strings: `Cow<str>`, `&str`, `&RawStr`, `String`
///
///     _Limited by the `string` [data limit]._
///
///     Reads the body data into a string via [`DataStream::into_string()`].
///
///     - **Fails:** If the body data is not valid UTF-8 or on I/O errors while
///     reading. The error type is [`io::Error`].
///
///     - **Succeeds:** If the body data _is_ valid UTF-8. If the limit is
///     exceeded, the string is truncated to the limit.
///
///     - **Forwards:** Never.
///
///   * Bytes: `&[u8]`, `Vec<u8>`
///
///     _Limited by the `bytes` [data limit]._
///
///     Reads the body data into a byte vector via [`DataStream::into_bytes()`].
///
///     - **Fails:** On I/O errors while reading. The error type is
///     [`io::Error`].
///
///     - **Succeeds:** As long as no I/O error occurs. If the limit is
///     exceeded, the slice is truncated to the limit.
///
///     - **Forwards:** Never.
///
///   * [`TempFile`](crate::fs::TempFile)
///
///     _Limited by the `file` and/or `file/$ext` [data limit]._
///
///     Streams the body data directly into a temporary file. The data is never
///     buffered in memory.
///
///     - **Fails:** On I/O errors while reading data or creating the temporary
///     file. The error type is [`io::Error`].
///
///     - **Succeeds:** As long as no I/O error occurs and the temporary file
///     could be created. If the limit is exceeded, only data up to the limit is
///     read and subsequently written.
///
///     - **Forwards:** Never.
///
///   * Deserializers: [`Json<T>`], [`MsgPack<T>`]
///
///     _Limited by the `json`, `msgpack` [data limit], respectively._
///
///     Reads up to the configured limit and deserializes the read data into `T`
///     using the respective format's parser.
///
///     - **Fails:** On I/O errors while reading the data, or if the data fails
///     to parse as a `T` according to the deserializer. The error type for
///     `Json` is [`json::Error`](crate::serde::json::Error) and the error type
///     for `MsgPack` is [`msgpack::Error`](crate::serde::msgpack::Error).
///
///     - **Succeeds:** As long as no I/O error occurs and the (limited) body
///     data was successfully deserialized as a `T`.
///
///     - **Forwards:** Never.
///
///   * Forms: [`Form<T>`]
///
///     _Limited by the `form` or `data-form` [data limit]._
///
///     Parses the incoming data stream into fields according to Rocket's [field
///     wire format], pushes each field to `T`'s [`FromForm`] [push parser], and
///     finalizes the form. Parsing is done on the stream without reading the
///     data into memory. If the request has as a [`ContentType::Form`], the
///     `form` limit is applied, otherwise if the request has a
///     [`ContentType::FormData`], the `data-form` limit is applied.
///
///     - **Fails:** On I/O errors while reading the data, or if the data fails
///     to parse as a `T` according to its `FromForm` implementation. The errors
///     are collected into an [`Errors`](crate::form::Errors), the error type.
///
///     - **Succeeds:** As long as no I/O error occurs and the (limited) body
///     data was successfully parsed as a `T`.
///
///     - **Forwards:** If the request's `Content-Type` is neither
///     [`ContentType::Form`] nor [`ContentType::FormData`].
///
///   * `Option<T>`
///
///     Forwards to `T`'s `FromData` implementation, capturing the outcome.
///
///     - **Fails:** Never.
///
///     - **Succeeds:** Always. If `T`'s `FromData` implementation succeeds, the
///     parsed value is returned in `Some`. If its implementation forwards or
///     fails, `None` is returned.
///
///     - **Forwards:** Never.
///
///   * `Result<T, T::Error>`
///
///     Forwards to `T`'s `FromData` implementation, capturing the outcome.
///
///     - **Fails:** Never.
///
///     - **Succeeds:** If `T`'s `FromData` implementation succeeds or fails. If
///     it succeeds, the value is returned in `Ok`. If it fails, the error value
///     is returned in `Err`.
///
///     - **Forwards:** If `T`'s implementation forwards.
///
///   * [`Capped<T>`]
///
///     Forwards to `T`'s `FromData` implementation, recording whether the data
///     was truncated (a.k.a. capped) due to `T`'s limit being exceeded.
///
///     - **Fails:** If `T`'s implementation fails.
///     - **Succeeds:** If `T`'s implementation succeeds.
///     - **Forwards:** If `T`'s implementation forwards.
///
/// [data limit]: crate::data::Limits#built-in-limits
/// [`DataStream::into_string()`]: crate::data::DataStream::into_string()
/// [`DataStream::into_bytes()`]: crate::data::DataStream::into_bytes()
/// [`io::Error`]: std::io::Error
/// [`Json<T>`]: crate::serde::json::Json
/// [`MsgPack<T>`]: crate::serde::msgpack::MsgPack
/// [`Form<T>`]: crate::form::Form
/// [field wire format]: crate::form#field-wire-format
/// [`FromForm`]: crate::form::FromForm
/// [push parser]: crate::form::FromForm#push-parsing
/// [`ContentType::Form`]: crate::http::ContentType::Form
/// [`ContentType::FormData`]: crate::http::ContentType::FormData
///
/// ## Async Trait
///
/// [`FromData`] is an _async_ trait. Implementations of `FromData` must be
/// decorated with an attribute of `#[rocket::async_trait]`:
///
/// ```rust
/// use rocket::request::Request;
/// use rocket::data::{self, Data, FromData};
/// # struct MyType;
/// # type MyError = String;
///
/// #[rocket::async_trait]
/// impl<'r> FromData<'r> for MyType {
///     type Error = MyError;
///
///     async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
///         /* .. */
///         # unimplemented!()
///     }
/// }
/// ```
///
/// # Example
///
/// Say that you have a custom type, `Person`:
///
/// ```rust
/// struct Person<'r> {
///     name: &'r str,
///     age: u16
/// }
/// ```
///
/// `Person` has a custom serialization format, so the built-in `Json` type
/// doesn't suffice. The format is `<name>:<age>` with `Content-Type:
/// application/x-person`. You'd like to use `Person` as a data guard, so that
/// you can retrieve it directly from a client's request body:
///
/// ```rust
/// # use rocket::post;
/// # type Person<'r> = &'r rocket::http::RawStr;
/// #[post("/person", data = "<person>")]
/// fn person(person: Person<'_>) -> &'static str {
///     "Saved the new person to the database!"
/// }
/// ```
///
/// A `FromData` implementation for such a type might look like:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person<'r> { name: &'r str, age: u16 }
/// #
/// use rocket::request::{self, Request};
/// use rocket::data::{self, Data, FromData, ToByteUnit};
/// use rocket::http::{Status, ContentType};
/// use rocket::outcome::Outcome;
///
/// #[derive(Debug)]
/// enum Error {
///     TooLarge,
///     NoColon,
///     InvalidAge,
///     Io(std::io::Error),
/// }
///
/// #[rocket::async_trait]
/// impl<'r> FromData<'r> for Person<'r> {
///     type Error = Error;
///
///     async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
///         use Error::*;
///
///         // Ensure the content type is correct before opening the data.
///         let person_ct = ContentType::new("application", "x-person");
///         if req.content_type() != Some(&person_ct) {
///             return Outcome::Forward((data, Status::UnsupportedMediaType));
///         }
///
///         // Use a configured limit with name 'person' or fallback to default.
///         let limit = req.limits().get("person").unwrap_or(256.bytes());
///
///         // Read the data into a string.
///         let string = match data.open(limit).into_string().await {
///             Ok(string) if string.is_complete() => string.into_inner(),
///             Ok(_) => return Outcome::Error((Status::PayloadTooLarge, TooLarge)),
///             Err(e) => return Outcome::Error((Status::InternalServerError, Io(e))),
///         };
///
///         // We store `string` in request-local cache for long-lived borrows.
///         let string = request::local_cache!(req, string);
///
///         // Split the string into two pieces at ':'.
///         let (name, age) = match string.find(':') {
///             Some(i) => (&string[..i], &string[(i + 1)..]),
///             None => return Outcome::Error((Status::UnprocessableEntity, NoColon)),
///         };
///
///         // Parse the age.
///         let age: u16 = match age.parse() {
///             Ok(age) => age,
///             Err(_) => return Outcome::Error((Status::UnprocessableEntity, InvalidAge)),
///         };
///
///         Outcome::Success(Person { name, age })
///     }
/// }
///
/// // The following routes now typecheck...
///
/// #[post("/person", data = "<person>")]
/// fn person(person: Person<'_>) { /* .. */ }
///
/// #[post("/person", data = "<person>")]
/// fn person2(person: Result<Person<'_>, Error>) { /* .. */ }
///
/// #[post("/person", data = "<person>")]
/// fn person3(person: Option<Person<'_>>) { /* .. */ }
///
/// #[post("/person", data = "<person>")]
/// fn person4(person: Person<'_>) -> &str {
///     // Note that this is only possible because the data in `person` live
///     // as long as the request through request-local cache.
///     person.name
/// }
/// ```
#[crate::async_trait]
pub trait FromData<'r>: Sized {
    /// The associated error to be returned when the guard fails.
    type Error: Send + std::fmt::Debug;

    /// Asynchronously validates, parses, and converts an instance of `Self`
    /// from the incoming request body data.
    ///
    /// If validation and parsing succeeds, an outcome of `Success` is returned.
    /// If the data is not appropriate given the type of `Self`, `Forward` is
    /// returned. If parsing fails, `Error` is returned.
    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self>;
}

use crate::data::Capped;

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<String> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let limit = req.limits().get("string").unwrap_or(Limits::STRING);
        data.open(limit).into_string().await.or_error(Status::BadRequest)
    }
}

impl_strict_from_data_from_capped!(String);

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<&'r str> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let capped = try_outcome!(<Capped<String>>::from_data(req, data).await);
        let string = capped.map(|s| local_cache!(req, s));
        Success(string)
    }
}

impl_strict_from_data_from_capped!(&'r str);

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<&'r RawStr> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let capped = try_outcome!(<Capped<String>>::from_data(req, data).await);
        let raw = capped.map(|s| RawStr::new(local_cache!(req, s)));
        Success(raw)
    }
}

impl_strict_from_data_from_capped!(&'r RawStr);

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<std::borrow::Cow<'_, str>> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let capped = try_outcome!(<Capped<String>>::from_data(req, data).await);
        Success(capped.map(|s| s.into()))
    }
}

impl_strict_from_data_from_capped!(std::borrow::Cow<'_, str>);

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<&'r [u8]> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let capped = try_outcome!(<Capped<Vec<u8>>>::from_data(req, data).await);
        let raw = capped.map(|b| local_cache!(req, b));
        Success(raw)
    }
}

impl_strict_from_data_from_capped!(&'r [u8]);

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<Vec<u8>> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let limit = req.limits().get("bytes").unwrap_or(Limits::BYTES);
        data.open(limit).into_bytes().await.or_error(Status::BadRequest)
    }
}

impl_strict_from_data_from_capped!(Vec<u8>);

#[crate::async_trait]
impl<'r> FromData<'r> for Data<'r> {
    type Error = std::convert::Infallible;

    async fn from_data(_: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        Success(data)
    }
}

#[crate::async_trait]
impl<'r, T: FromData<'r> + 'r> FromData<'r> for Result<T, T::Error> {
    type Error = std::convert::Infallible;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        match T::from_data(req, data).await {
            Success(v) => Success(Ok(v)),
            Error((_, e)) => Success(Err(e)),
            Forward(d) => Forward(d),
        }
    }
}

#[crate::async_trait]
impl<'r, T: FromData<'r>> FromData<'r> for Option<T> {
    type Error = std::convert::Infallible;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        match T::from_data(req, data).await {
            Success(v) => Success(Some(v)),
            Error(..) | Forward(..) => Success(None),
        }
    }
}
