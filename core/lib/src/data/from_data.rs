use std::borrow::Borrow;

use futures::future::BoxFuture;
use futures::future::{ready, FutureExt};

use crate::outcome::{self, IntoOutcome};
use crate::outcome::Outcome::*;
use crate::http::Status;
use crate::request::Request;
use crate::data::Data;

/// Type alias for the `Outcome` of a `FromTransformedData` conversion.
pub type Outcome<S, E> = outcome::Outcome<S, (Status, E), Data>;

impl<S, E> IntoOutcome<S, (Status, E), Data> for Result<S, E> {
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

/// Indicates how incoming data should be transformed before being parsed and
/// validated by a data guard.
///
/// See the documentation for [`FromTransformedData`] for usage details.
pub enum Transform<T, B = T> {
    /// Indicates that data should be or has been transformed into the
    /// [`FromTransformedData::Owned`] variant.
    Owned(T),

    /// Indicates that data should be or has been transformed into the
    /// [`FromTransformedData::Borrowed`] variant.
    Borrowed(B)
}

impl<T, B> Transform<T, B> {
    /// Returns the `Owned` value if `self` is `Owned`.
    ///
    /// # Panics
    ///
    /// Panics if `self` is `Borrowed`.
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::Transform;
    ///
    /// let owned: Transform<usize, &[usize]> = Transform::Owned(10);
    /// assert_eq!(owned.owned(), 10);
    /// ```
    #[inline]
    pub fn owned(self) -> T {
        match self {
            Transform::Owned(val) => val,
            Transform::Borrowed(_) => panic!("Transform::owned() called on Borrowed"),
        }
    }

    /// Returns the `Borrowed` value if `self` is `Borrowed`.
    ///
    /// # Panics
    ///
    /// Panics if `self` is `Owned`.
    ///
    /// ```rust
    /// use rocket::data::Transform;
    ///
    /// let borrowed: Transform<usize, &[usize]> = Transform::Borrowed(&[10]);
    /// assert_eq!(borrowed.borrowed(), &[10]);
    /// ```
    #[inline]
    pub fn borrowed(self) -> B {
        match self {
            Transform::Borrowed(val) => val,
            Transform::Owned(_) => panic!("Transform::borrowed() called on Owned"),
        }
    }
}

/// Type alias to the `outcome` input type of [`FromTransformedData::from_data`].
///
/// This is a hairy type, but the gist is that this is a [`Transform`] where,
/// for a given `T: FromTransformedData`:
///
///   * The `Owned` variant is an `Outcome` whose `Success` value is of type
///     [`FromTransformedData::Owned`].
///
///   * The `Borrowed` variant is an `Outcome` whose `Success` value is a borrow
///     of type [`FromTransformedData::Borrowed`].
///
///   * In either case, the `Outcome`'s `Failure` variant is a value of type
///     [`FromTransformedData::Error`].
pub type Transformed<'a, T> =
    Transform<
        Outcome<<T as FromTransformedData<'a>>::Owned, <T as FromTransformedData<'a>>::Error>,
        Outcome<&'a <T as FromTransformedData<'a>>::Borrowed, <T as FromTransformedData<'a>>::Error>
    >;

/// Type alias to the `Future` returned by [`FromTransformedData::transform`].
pub type TransformFuture<'fut, T, E> = BoxFuture<'fut, Transform<Outcome<T, E>>>;

/// Type alias to the `Future` returned by [`FromTransformedData::from_data`].
pub type FromDataFuture<'fut, T, E> = BoxFuture<'fut, Outcome<T, E>>;

/// Trait implemented by data guards to derive a value from request body data.
///
/// # Data Guards
///
/// A data guard is a [request guard] that operates on a request's body data.
/// Data guards validate, parse, and optionally convert request body data.
/// Validation and parsing/conversion is implemented through
/// `FromTransformedData`. In other words, every type that implements
/// `FromTransformedData` is a data guard.
///
/// Data guards are used as the target of the `data` route attribute parameter.
/// A handler can have at most one data guard.
///
/// For many data guards, implementing [`FromData`] will be simpler and
/// sufficient. All types that implement `FromData` automatically implement
/// `FromTransformedData`. Thus, when possible, prefer to implement [`FromData`]
/// instead of `FromTransformedData`.
///
/// [request guard]: crate::request::FromRequest
///
/// ## Example
///
/// In the example below, `var` is used as the argument name for the data guard
/// type `DataGuard`. When the `submit` route matches, Rocket will call the
/// `FromTransformedData` implementation for the type `T`. The handler will only be called
/// if the guard returns successfully.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type DataGuard = rocket::data::Data;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: DataGuard) { /* ... */ }
/// # fn main() { }
/// ```
///
/// # Transforming
///
/// Data guards can optionally _transform_ incoming data before processing it
/// via an implementation of the [`FromTransformedData::transform()`] method.
/// This is useful when a data guard requires or could benefit from a reference
/// to body data as opposed to an owned version. If a data guard has no need to
/// operate on a reference to body data, [`FromData`] should be implemented
/// instead; it is simpler to implement and less error prone. All types that
/// implement `FromData` automatically implement `FromTransformedData`.
///
/// When exercising a data guard, Rocket first calls the guard's
/// [`FromTransformedData::transform()`] method and awaits on the returned
/// future, then calls the guard's [`FromTransformedData::from_data()`] method
/// and awaits on that returned future. Rocket stores data returned by
/// [`FromTransformedData::transform()`] on the stack. If `transform` returns a
/// [`Transform::Owned`], Rocket moves the data back to the data guard in the
/// subsequent `from_data` call as a `Transform::Owned`. If instead `transform`
/// returns a [`Transform::Borrowed`] variant, Rocket calls `borrow()` on the
/// owned value, producing a borrow of the associated
/// [`FromTransformedData::Borrowed`] type and passing it as a
/// `Transform::Borrowed`.
///
/// ## Example
///
/// Consider a data guard type that wishes to hold a slice to two different
/// parts of the incoming data:
///
/// ```rust
/// struct Name<'a> {
///     first: &'a str,
///     last: &'a str
/// }
/// ```
///
/// Without the ability to transform into a borrow, implementing such a data
/// guard would be impossible. With transformation, however, we can instruct
/// Rocket to produce a borrow to a `Data` that has been transformed into a
/// `String` (an `&str`).
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[derive(Debug)]
/// # struct Name<'a> { first: &'a str, last: &'a str, }
/// use std::io::{self, Read};
///
/// use rocket::Request;
/// use rocket::data::{Data, Outcome, FromDataFuture, ByteUnit};
/// use rocket::data::{FromTransformedData, Transform, Transformed, TransformFuture};
/// use rocket::http::Status;
///
/// const NAME_LIMIT: ByteUnit = ByteUnit::Byte(256);
///
/// enum NameError {
///     Io(io::Error),
///     Parse
/// }
///
/// impl<'r> FromTransformedData<'r> for Name<'r> {
///     type Error = NameError;
///     type Owned = String;
///     type Borrowed = str;
///
///     fn transform(_: &'r Request, data: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
///         Box::pin(async move {
///             let outcome = match data.open(NAME_LIMIT).stream_to_string().await {
///                 Ok(string) => Outcome::Success(string),
///                 Err(e) => Outcome::Failure((Status::InternalServerError, NameError::Io(e)))
///             };
///
///             // Returning `Borrowed` here means we get `Borrowed` in `from_data`.
///             Transform::Borrowed(outcome)
///         })
///     }
///
///     fn from_data(_: &'r Request, outcome: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
///         Box::pin(async move {
///             // Retrieve a borrow to the now transformed `String` (an &str).
///             // This is only correct because we know we _always_ return a
///             // `Borrowed` from `transform` above.
///             let string = try_outcome!(outcome.borrowed());
///
///             // Perform a crude, inefficient parse.
///             let splits: Vec<&str> = string.split(" ").collect();
///             if splits.len() != 2 || splits.iter().any(|s| s.is_empty()) {
///                 return Outcome::Failure((Status::UnprocessableEntity, NameError::Parse));
///             }
///
///             // Return successfully.
///             Outcome::Success(Name { first: splits[0], last: splits[1] })
///         })
///     }
/// }
/// # #[post("/person", data = "<person>")]
/// # fn person(person: Name) {  }
/// # #[post("/person", data = "<person>")]
/// # fn person2(person: Result<Name, NameError>) {  }
/// # fn main() {  }
/// ```
///
/// # Outcomes
///
/// The returned [`Outcome`] of a `from_data` call determines how the incoming
/// request will be processed.
///
/// * **Success**(S)
///
///   If the `Outcome` is [`Success`], then the `Success` value will be used as
///   the value for the data parameter.  As long as all other parsed types
///   succeed, the request will be handled by the requesting handler.
///
/// * **Failure**(Status, E)
///
///   If the `Outcome` is [`Failure`], the request will fail with the given
///   status code and error. The designated error [`Catcher`](crate::Catcher) will be
///   used to respond to the request. Note that users can request types of
///   `Result<S, E>` and `Option<S>` to catch `Failure`s and retrieve the error
///   value.
///
/// * **Forward**(Data)
///
///   If the `Outcome` is [`Forward`], the request will be forwarded to the next
///   matching request. This requires that no data has been read from the `Data`
///   parameter. Note that users can request an `Option<S>` to catch `Forward`s.
///
/// # Provided Implementations
///
/// Rocket implements `FromTransformedData` for several built-in types. Their behavior is
/// documented here.
///
///   * **Data**
///
///     The identity implementation; simply returns [`Data`] directly.
///
///     _This implementation always returns successfully._
///
///   * **Option&lt;T>** _where_ **T: FromTransformedData**
///
///     The type `T` is derived from the incoming data using `T`'s `FromTransformedData`
///     implementation. If the derivation is a `Success`, the derived value is
///     returned in `Some`. Otherwise, a `None` is returned.
///
///     _This implementation always returns successfully._
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromTransformedData**
///
///     The type `T` is derived from the incoming data using `T`'s `FromTransformedData`
///     implementation. If derivation is a `Success`, the value is returned in
///     `Ok`. If the derivation is a `Failure`, the error value is returned in
///     `Err`. If the derivation is a `Forward`, the request is forwarded.
///
///   * **String**
///
///     **Note:** _An implementation of `FromTransformedData` for `String` is only available
///     when compiling in debug mode!_
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
///     **Note:** _An implementation of `FromTransformedData` for `Vec<u8>` is only
///     available when compiling in debug mode!_
///
///     Reads the entire request body into a `Vec<u8>`. If reading fails,
///     returns a `Failure` with the corresponding `io::Error`.
///
///     **WARNING:** Do **not** use this implementation for anything _but_
///     debugging. This is because the implementation reads the entire body into
///     memory; since the user controls the size of the body, this is an obvious
///     vector for a denial of service attack.
///
/// # Simplified `FromTransformedData`
///
/// For an example of a type that wouldn't require transformation, see the
/// [`FromData`] documentation.
pub trait FromTransformedData<'r>: Sized {
    /// The associated error to be returned when the guard fails.
    type Error: Send;

    /// The owned type returned from [`FromTransformedData::transform()`].
    ///
    /// The trait bounds ensures that it is is possible to borrow an
    /// `&Self::Borrowed` from a value of this type.
    type Owned: Borrow<Self::Borrowed>;

    /// The _borrowed_ type consumed by [`FromTransformedData::from_data()`] when
    /// [`FromTransformedData::transform()`] returns a [`Transform::Borrowed`].
    ///
    /// If [`FromTransformedData::from_data()`] returns a [`Transform::Owned`], this
    /// associated type should be set to `Self::Owned`.
    type Borrowed: ?Sized;

    /// Asynchronously transforms `data` into a value of type `Self::Owned`.
    ///
    /// If the returned future resolves to `Transform::Owned(Self::Owned)`, then
    /// `from_data` should subsequently be called with a `data` value of
    /// `Transform::Owned(Self::Owned)`. If the future resolves to
    /// `Transform::Borrowed(Self::Owned)`, `from_data` should subsequently be
    /// called with a `data` value of `Transform::Borrowed(&Self::Borrowed)`. In
    /// other words, the variant of `Transform` returned from this method is
    /// used to determine which variant of `Transform` should be passed to the
    /// `from_data` method. Rocket _always_ makes the subsequent call correctly.
    ///
    /// It is very unlikely that a correct implementation of this method is
    /// capable of returning either of an `Owned` or `Borrowed` variant.
    /// Instead, this method should return exactly _one_ of these variants.
    ///
    /// If transformation succeeds, an outcome of `Success` is returned.
    /// If the data is not appropriate given the type of `Self`, `Forward` is
    /// returned. On failure, `Failure` is returned.
    fn transform(request: &'r Request<'_>, data: Data) -> TransformFuture<'r, Self::Owned, Self::Error>;

    /// Asynchronously validates, parses, and converts the incoming request body
    /// data into an instance of `Self`.
    ///
    /// If validation and parsing succeeds, an outcome of `Success` is returned.
    /// If the data is not appropriate given the type of `Self`, `Forward` is
    /// returned. If parsing or validation fails, `Failure` is returned.
    ///
    /// # Example
    ///
    /// When implementing this method, you rarely need to destruct the `outcome`
    /// parameter. Instead, the first line of the method should be one of the
    /// following:
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # use rocket::data::{Data, FromTransformedData, Transformed, Outcome};
    /// # fn f<'a>(outcome: Transformed<'a, Data>) -> Outcome<Data, <Data as FromTransformedData<'a>>::Error> {
    /// // If `Owned` was returned from `transform`:
    /// let data = try_outcome!(outcome.owned());
    /// # unimplemented!()
    /// # }
    ///
    /// # fn g<'a>(outcome: Transformed<'a, Data>) -> Outcome<Data, <Data as FromTransformedData<'a>>::Error> {
    /// // If `Borrowed` was returned from `transform`:
    /// let data = try_outcome!(outcome.borrowed());
    /// # unimplemented!()
    /// # }
    /// ```
    fn from_data(request: &'r Request<'_>, outcome: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error>;
}

/// The identity implementation of `FromTransformedData`. Always returns `Success`.
impl<'r> FromTransformedData<'r> for Data {
    type Error = std::convert::Infallible;
    type Owned = Data;
    type Borrowed = Data;

    #[inline(always)]
    fn transform(_: &'r Request<'_>, data: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        Box::pin(ready(Transform::Owned(Success(data))))
    }

    #[inline(always)]
    fn from_data(_: &'r Request<'_>, outcome: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
        Box::pin(ready(outcome.owned()))
    }
}

/// A variant of [`FromTransformedData`] for data guards that don't require
/// transformations.
///
/// When transformation of incoming data isn't required, data guards should
/// implement this trait instead of [`FromTransformedData`]. Any type that
/// implements `FromData` automatically implements `FromTransformedData`. For a
/// description of data guards, see the [`FromTransformedData`] documentation.
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
/// impl FromData for MyType {
///     type Error = MyError;
///
///     async fn from_data(req: &Request<'_>, data: Data) -> data::Outcome<Self, MyError> {
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
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// `Person` has a custom serialization format, so the built-in `Json` type
/// doesn't suffice. The format is `<name>:<age>` with `Content-Type:
/// application/x-person`. You'd like to use `Person` as a `FromTransformedData` type so
/// that you can retrieve it directly from a client's request body:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type Person = rocket::data::Data;
/// #[post("/person", data = "<person>")]
/// fn person(person: Person) -> &'static str {
///     "Saved the new person to the database!"
/// }
/// ```
///
/// A `FromData` implementation allowing this looks like:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person { name: String, age: u16 }
/// #
/// use std::io::Read;
///
/// use rocket::{Request, Data};
/// use rocket::data::{self, Outcome, FromData, FromDataFuture, ByteUnit};
/// use rocket::http::{Status, ContentType};
/// use rocket::tokio::io::AsyncReadExt;
///
/// // Always use a limit to prevent DoS attacks.
/// const LIMIT: ByteUnit = ByteUnit::Byte(256);
///
/// #[rocket::async_trait]
/// impl FromData for Person {
///     type Error = String;
///
///     async fn from_data(req: &Request<'_>, data: Data) -> Outcome<Self, String> {
///         // Ensure the content type is correct before opening the data.
///         let person_ct = ContentType::new("application", "x-person");
///         if req.content_type() != Some(&person_ct) {
///             return Outcome::Forward(data);
///         }
///
///         // Read the data into a String.
///         let limit = req.limits().get("person").unwrap_or(LIMIT);
///         let string = match data.open(limit).stream_to_string().await {
///             Ok(string) => string,
///             Err(e) => return Outcome::Failure((Status::InternalServerError, format!("{}", e)))
///         };
///
///         // Split the string into two pieces at ':'.
///         let (name, age) = match string.find(':') {
///             Some(i) => (string[..i].to_string(), &string[(i + 1)..]),
///             None => return Outcome::Failure((Status::UnprocessableEntity, "':'".into()))
///         };
///
///         // Parse the age.
///         let age: u16 = match age.parse() {
///             Ok(age) => age,
///             Err(_) => return Outcome::Failure((Status::UnprocessableEntity, "Age".into()))
///         };
///
///         // Return successfully.
///         Outcome::Success(Person { name, age })
///     }
/// }
/// # #[post("/person", data = "<person>")]
/// # fn person(person: Person) {  }
/// # #[post("/person", data = "<person>")]
/// # fn person2(person: Result<Person, String>) {  }
/// # fn main() {  }
/// ```
#[crate::async_trait]
pub trait FromData: Sized {
    /// The associated error to be returned when the guard fails.
    type Error: Send + 'static;

    /// Asynchronously validates, parses, and converts an instance of `Self`
    /// from the incoming request body data.
    ///
    /// If validation and parsing succeeds, an outcome of `Success` is returned.
    /// If the data is not appropriate given the type of `Self`, `Forward` is
    /// returned. If parsing fails, `Failure` is returned.
    async fn from_data(request: &Request<'_>, data: Data) -> Outcome<Self, Self::Error>;
}

impl<'r, T: FromData + 'r> FromTransformedData<'r> for T {
    type Error = T::Error;
    type Owned = Data;
    type Borrowed = Data;

    #[inline(always)]
    fn transform(_: &'r Request<'_>, d: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        Box::pin(ready(Transform::Owned(Success(d))))
    }

    #[inline(always)]
    fn from_data(req: &'r Request<'_>, o: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
        match o.owned() {
            Success(data) => T::from_data(req, data),
            _ => unreachable!(),
        }
    }
}

impl<'r, T: FromTransformedData<'r> + 'r> FromTransformedData<'r> for Result<T, T::Error> {
    type Error = T::Error;
    type Owned = T::Owned;
    type Borrowed = T::Borrowed;

    #[inline(always)]
    fn transform(r: &'r Request<'_>, d: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        T::transform(r, d)
    }

    #[inline(always)]
    fn from_data(r: &'r Request<'_>, o: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
        Box::pin(T::from_data(r, o).map(|x| match x {
            Success(val) => Success(Ok(val)),
            Forward(data) => Forward(data),
            Failure((_, e)) => Success(Err(e)),
        }))
    }
}

impl<'r, T: FromTransformedData<'r> + 'r> FromTransformedData<'r> for Option<T> {
    type Error = T::Error;
    type Owned = T::Owned;
    type Borrowed = T::Borrowed;

    #[inline(always)]
    fn transform(r: &'r Request<'_>, d: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        T::transform(r, d)
    }

    #[inline(always)]
    fn from_data(r: &'r Request<'_>, o: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
        Box::pin(T::from_data(r, o).map(|x| match x {
            Success(val) => Success(Some(val)),
            Failure(_) | Forward(_) => Success(None),
        }))
    }
}

#[cfg(debug_assertions)]
use crate::data::ByteUnit;

#[crate::async_trait]
#[cfg(debug_assertions)]
impl FromData for String {
    type Error = std::io::Error;

    #[inline(always)]
    async fn from_data(_: &Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        match data.open(ByteUnit::max_value()).stream_to_string().await {
            Ok(string) => Success(string),
            Err(e) => Failure((Status::BadRequest, e)),
        }
    }
}

#[crate::async_trait]
#[cfg(debug_assertions)]
impl FromData for Vec<u8> {
    type Error = std::io::Error;

    #[inline(always)]
    async fn from_data(_: &Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        use tokio::io::AsyncReadExt;

        let mut stream = data.open(ByteUnit::max_value());
        let mut buf = Vec::new();
        match stream.read_to_end(&mut buf).await {
            Ok(_) => Success(buf),
            Err(e) => Failure((Status::BadRequest, e)),
        }
    }
}
