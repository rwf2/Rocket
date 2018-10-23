use std::borrow::Borrow;

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

/// Indicates how incoming data should be transformed before being parsed and
/// validated by a data guard.
///
/// See the documentation for [`FromData`] for usage details.
pub enum Transform<T, B = T> {
    /// Indicates that data should be or has been transformed into the
    /// [`FromData::Owned`] variant.
    Owned(T),

    /// Indicates that data should be or has been transformed into the
    /// [`FromData::Borrowed`] variant.
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

/// Type alias to the `outcome` input type of [`FromData::from_data`].
///
/// This is a hairy type, but the gist is that this is a [`Transform`] where,
/// for a given `T: FromData`:
///
///   * The `Owned` variant is an `Outcome` whose `Success` value is of type
///     [`FromData::Owned`].
///
///   * The `Borrowed` variant is an `Outcome` whose `Success` value is a borrow
///     of type [`FromData::Borrowed`].
///
///   * In either case, the `Outcome`'s `Failure` variant is a value of type
///     [`FromData::Error`].
pub type Transformed<'a, T> =
    Transform<
        Outcome<<T as FromData<'a>>::Owned, <T as FromData<'a>>::Error>,
        Outcome<&'a <T as FromData<'a>>::Borrowed, <T as FromData<'a>>::Error>
    >;

/// Trait implemented by data guards to derive a value from request body data.
///
/// # Data Guards
///
/// A data guard is a [request guard] that operates on a request's body data.
/// Data guards validate, parse, and optionally convert request body data.
/// Validation and parsing/conversion is implemented through `FromData`. In
/// other words, every type that implements `FromData` is a data guard.
///
/// Data guards are used as the target of the `data` route attribute parameter.
/// A handler can have at most one data guard.
///
/// For many data guards, implementing [`FromDataSimple`] will be simpler and
/// sufficient. All types that implement `FromDataSimple` automatically
/// implement `FromData`. Thus, when possible, prefer to implement
/// [`FromDataSimple`] instead of `FromData`.
///
/// [request guard]: ::request::FromRequest
///
/// ## Example
///
/// In the example below, `var` is used as the argument name for the data guard
/// type `DataGuard`. When the `submit` route matches, Rocket will call the
/// `FromData` implementation for the type `T`. The handler will only be called
/// if the guard returns successfully.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # type DataGuard = ::rocket::data::Data;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: DataGuard) { /* ... */ }
/// # fn main() { }
/// ```
///
/// # Transforming
///
/// Data guards can optionally _transform_ incoming data before processing it
/// via an implementation of the [`FromData::transform()`] method. This is
/// useful when a data guard requires or could benefit from a reference to body
/// data as opposed to an owned version. If a data guard has no need to operate
/// on a reference to body data, [`FromDataSimple`] should be implemented
/// instead; it is simpler to implement and less error prone. All types that
/// implement `FromDataSimple` automatically implement `FromData`.
///
/// When exercising a data guard, Rocket first calls the guard's
/// [`FromData::transform()`] method and then subsequently calls the guard's
/// [`FromData::from_data()`] method. Rocket stores data returned by
/// [`FromData::transform()`] on the stack. If `transform` returns a
/// [`Transform::Owned`], Rocket moves the data back to the data guard in the
/// subsequent `from_data` call as a `Transform::Owned`. If instead `transform`
/// returns a [`Transform::Borrowed`] variant, Rocket calls `borrow()` on the
/// owned value, producing a borrow of the associated [`FromData::Borrowed`]
/// type and passing it as a `Transform::Borrowed`.
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
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # #[derive(Debug)]
/// # struct Name<'a> { first: &'a str, last: &'a str, }
/// use std::io::{self, Read};
///
/// use rocket::{Request, Data, Outcome::*};
/// use rocket::data::{FromData, Outcome, Transform, Transformed};
/// use rocket::http::Status;
///
/// const NAME_LIMIT: u64 = 256;
///
/// enum NameError {
///     Io(io::Error),
///     Parse
/// }
///
/// impl<'a> FromData<'a> for Name<'a> {
///     type Error = NameError;
///     type Owned = String;
///     type Borrowed = str;
///
///     fn transform(_: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
///         let mut stream = data.open().take(NAME_LIMIT);
///         let mut string = String::with_capacity((NAME_LIMIT / 2) as usize);
///         let outcome = match stream.read_to_string(&mut string) {
///             Ok(_) => Success(string),
///             Err(e) => Failure((Status::InternalServerError, NameError::Io(e)))
///         };
///
///         // Returning `Borrowed` here means we get `Borrowed` in `from_data`.
///         Transform::Borrowed(outcome)
///     }
///
///     fn from_data(_: &Request, outcome: Transformed<'a, Self>) -> Outcome<Self, Self::Error> {
///         // Retrieve a borrow to the now transformed `String` (an &str). This
///         // is only correct because we know we _always_ return a `Borrowed` from
///         // `transform` above.
///         let string = outcome.borrowed()?;
///
///         // Perform a crude, inefficient parse.
///         let splits: Vec<&str> = string.split(" ").collect();
///         if splits.len() != 2 || splits.iter().any(|s| s.is_empty()) {
///             return Failure((Status::UnprocessableEntity, NameError::Parse));
///         }
///
///         // Return successfully.
///         Success(Name { first: splits[0], last: splits[1] })
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
///   status code and error. The designated error [`Catcher`](::Catcher) will be
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
/// Rocket implements `FromData` for several built-in types. Their behavior is
/// documented here.
///
///   * **Data**
///
///     The identity implementation; simply returns [`Data`] directly.
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
///     **Note:** _An implementation of `FromData` for `String` is only available
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
///     **Note:** _An implementation of `FromData` for `Vec<u8>` is only
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
/// # Simplified `FromData`
///
/// For an example of a type that wouldn't require transformation, see the
/// [`FromDataSimple`] documentation.
pub trait FromData<'a>: Sized {
    /// The associated error to be returned when the guard fails.
    type Error;

    /// The owned type returned from [`FromData::transform()`].
    ///
    /// The trait bounds ensures that it is is possible to borrow an
    /// `&Self::Borrowed` from a value of this type.
    type Owned: Borrow<Self::Borrowed>;

    /// The _borrowed_ type consumed by [`FromData::from_data()`] when
    /// [`FromData::transform()`] returns a [`Transform::Borrowed`].
    ///
    /// If [`FromData::from_data()`] returns a [`Transform::Owned`], this
    /// associated type should be set to `Self::Owned`.
    type Borrowed: ?Sized;

    /// Transforms `data` into a value of type `Self::Owned`.
    ///
    /// If this method returns a `Transform::Owned(Self::Owned)`, then
    /// `from_data` should subsequently be called with a `data` value of
    /// `Transform::Owned(Self::Owned)`. If this method returns a
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
    fn transform(request: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>>;

    /// Validates, parses, and converts the incoming request body data into an
    /// instance of `Self`.
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
    /// # use rocket::data::{Data, FromData, Transformed, Outcome};
    /// # fn f<'a>(outcome: Transformed<'a, Data>) -> Outcome<Data, <Data as FromData<'a>>::Error> {
    /// // If `Owned` was returned from `transform`:
    /// let data = outcome.owned()?;
    /// # unimplemented!()
    /// # }
    ///
    /// # fn g<'a>(outcome: Transformed<'a, Data>) -> Outcome<Data, <Data as FromData<'a>>::Error> {
    /// // If `Borrowed` was returned from `transform`:
    /// let data = outcome.borrowed()?;
    /// # unimplemented!()
    /// # }
    /// ```
    fn from_data(request: &Request, outcome: Transformed<'a, Self>) -> Outcome<Self, Self::Error>;
}

/// The identity implementation of `FromData`. Always returns `Success`.
impl<'f> FromData<'f> for Data {
    type Error = !;
    type Owned = Data;
    type Borrowed = Data;

    #[inline(always)]
    fn transform(_: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        Transform::Owned(Success(data))
    }

    #[inline(always)]
    fn from_data(_: &Request, outcome: Transformed<'f, Self>) -> Outcome<Self, Self::Error> {
        Success(outcome.owned()?)
    }
}

/// A simple, less complex variant of [`FromData`].
///
/// When transformation of incoming data isn't required, data guards should
/// implement this trait instead of [`FromData`]. Any type that implements
/// `FromDataSimple` automatically implements `FromData`. For a description of
/// data guards, see the [`FromData`] documentation.
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
/// application/x-person`. You'd like to use `Person` as a `FromData` type so
/// that you can retrieve it directly from a client's request body:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # type Person = ::rocket::data::Data;
/// #[post("/person", data = "<person>")]
/// fn person(person: Person) -> &'static str {
///     "Saved the new person to the database!"
/// }
/// ```
///
/// A `FromDataSimple` implementation allowing this looks like:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person { name: String, age: u16 }
/// #
/// use std::io::Read;
///
/// use rocket::{Request, Data, Outcome, Outcome::*};
/// use rocket::data::{self, FromDataSimple};
/// use rocket::http::{Status, ContentType};
///
/// // Always use a limit to prevent DoS attacks.
/// const LIMIT: u64 = 256;
///
/// impl FromDataSimple for Person {
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
///         if let Err(e) = data.open().take(LIMIT).read_to_string(&mut string) {
///             return Failure((Status::InternalServerError, format!("{:?}", e)));
///         }
///
///         // Split the string into two pieces at ':'.
///         let (name, age) = match string.find(':') {
///             Some(i) => (string[..i].to_string(), &string[(i + 1)..]),
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
///         Success(Person { name, age })
///     }
/// }
/// # #[post("/person", data = "<person>")]
/// # fn person(person: Person) {  }
/// # #[post("/person", data = "<person>")]
/// # fn person2(person: Result<Person, String>) {  }
/// # fn main() {  }
/// ```
pub trait FromDataSimple: Sized {
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

impl<'a, T: FromDataSimple> FromData<'a> for T {
    type Error = T::Error;
    type Owned = Data;
    type Borrowed = Data;

    #[inline(always)]
    fn transform(_: &Request, d: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        Transform::Owned(Success(d))
    }

    #[inline(always)]
    fn from_data(req: &Request, o: Transformed<'a, Self>) -> Outcome<Self, Self::Error> {
        T::from_data(req, o.owned()?)
    }
}

impl<'a, T: FromData<'a> + 'a> FromData<'a> for Result<T, T::Error> {
    type Error = T::Error;
    type Owned = T::Owned;
    type Borrowed = T::Borrowed;

    #[inline(always)]
    fn transform(r: &Request, d: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        T::transform(r, d)
    }

    #[inline(always)]
    fn from_data(r: &Request, o: Transformed<'a, Self>) -> Outcome<Self, Self::Error> {
        match T::from_data(r, o) {
            Success(val) => Success(Ok(val)),
            Forward(data) => Forward(data),
            Failure((_, e)) => Success(Err(e)),
        }
    }
}

impl<'a, T: FromData<'a> + 'a> FromData<'a> for Option<T> {
    type Error = T::Error;
    type Owned = T::Owned;
    type Borrowed = T::Borrowed;

    #[inline(always)]
    fn transform(r: &Request, d: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        T::transform(r, d)
    }

    #[inline(always)]
    fn from_data(r: &Request, o: Transformed<'a, Self>) -> Outcome<Self, Self::Error> {
        match T::from_data(r, o) {
            Success(val) => Success(Some(val)),
            Failure(_) | Forward(_) => Success(None),
        }
    }
}

#[cfg(debug_assertions)]
use std::io::{self, Read};

#[cfg(debug_assertions)]
impl FromDataSimple for String {
    type Error = io::Error;

    #[inline(always)]
    fn from_data(_: &Request, data: Data) -> Outcome<Self, Self::Error> {
        let mut string = String::new();
        match data.open().read_to_string(&mut string) {
            Ok(_) => Success(string),
            Err(e) => Failure((Status::BadRequest, e))
        }
    }
}

#[cfg(debug_assertions)]
impl FromDataSimple for Vec<u8> {
    type Error = io::Error;

    #[inline(always)]
    fn from_data(_: &Request, data: Data) -> Outcome<Self, Self::Error> {
        let mut bytes = Vec::new();
        match data.open().read_to_end(&mut bytes) {
            Ok(_) => Success(bytes),
            Err(e) => Failure((Status::BadRequest, e))
        }
    }
}
