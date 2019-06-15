use crate::request::{FormItems, FormItem, Form, LenientForm, FromForm};

/// Iterator over form items in a query string.
///
/// The `Query` type exists to separate, at the type level, _form_ form items
/// ([`FormItems`]) from _query_ form items (`Query`). A value of type `Query`
/// is passed in to implementations of the [`FromQuery`] trait by Rocket's code
/// generation for every trailing query parameter, `<params..>` below:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # use rocket::request::Form;
/// # #[derive(FromForm)] struct Q { foo: usize }
/// # type T = Form<Q>;
/// #
/// #[get("/user?<params..>")]
/// fn user(params: T) { /* ... */ }
/// # fn main() { }
/// ```
///
/// # Usage
///
/// A value of type `Query` can only be used as an iterator over values of type
/// [`FormItem`]. As such, its usage is equivalent to that of [`FormItems`], and
/// we refer you to its documentation for further details.
///
/// ## Example
///
/// ```rust
/// use rocket::request::Query;
///
/// # use rocket::request::FromQuery;
/// #
/// # struct MyType;
/// # type Result = std::result::Result<MyType, ()>;
/// #
/// # impl FromQuery<'_> for MyType {
/// #    type Error = ();
/// #
/// fn from_query(query: Query) -> Result {
///     for item in query {
///         println!("query key/value: ({}, {})", item.key, item.value);
///     }
///
///     // ...
/// #   Ok(MyType)
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Query<'q>(#[doc(hidden)] pub &'q [FormItem<'q>]);

impl<'q> Iterator for Query<'q> {
    type Item = FormItem<'q>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let next = self.0[0];
        self.0 = &self.0[1..];
        Some(next)
    }
}

/// Trait implemented by query guards to derive a value from a query string.
///
/// # Query Guards
///
/// A query guard operates on multiple items of a request's query string. It
/// validates and optionally converts a query string into another value.
/// Validation and parsing/conversion is implemented through `FromQuery`. In
/// other words, every type that implements `FromQuery` is a query guard.
///
/// Query guards are used as the target of trailing query parameters, which
/// syntactically take the form `<param..>` after a `?` in a route's path. For
/// example, the parameter `user` is a trailing query parameter in the following
/// route:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::request::Form;
///
/// #[derive(FromForm)]
/// struct User {
///     name: String,
///     account: usize,
/// }
///
/// #[get("/item?<id>&<user..>")]
/// fn item(id: usize, user: Form<User>) { /* ... */ }
/// # fn main() { }
/// ```
///
/// The `FromQuery` implementation of `Form<User>` will be passed in a [`Query`]
/// that iterates over all of the query items that don't have the key `id`
/// (because of the `<id>` dynamic query parameter). For posterity, note that
/// the `value` of an `id=value` item in a query string will be parsed as a
/// `usize` and passed in to `item` as `id`.
///
/// # Forwarding
///
/// If the conversion fails, signaled by returning an `Err` from a `FromQuery`
/// implementation, the incoming request will be forwarded to the next matching
/// route, if any. For instance, in the `item` route above, if a query string is
/// missing either a `name` or `account` key/value pair, or there is a query
/// item with a key that is not `id`, `name`, or `account`, the request will be
/// forwarded. Note that this strictness is imposed by the [`Form`] type. As an
/// example, using the [`LenientForm`] type instead would allow extra form items
/// to be ignored without forwarding. Alternatively, _not_ having a trailing
/// parameter at all would result in the same.
///
/// # Provided Implementations
///
/// Rocket implements `FromQuery` for several standard types. Their behavior is
/// documented here.
///
///   * **Form&lt;T>** _where_ **T: FromForm**
///
///     Parses the query as a strict form, where each key is mapped to a field
///     in `T`. See [`Form`] for more information.
///
///   * **LenientForm&lt;T>** _where_ **T: FromForm**
///
///     Parses the query as a lenient form, where each key is mapped to a field
///     in `T`. See [`LenientForm`] for more information.
///
///   * **Option&lt;T>** _where_ **T: FromQuery**
///
///     _This implementation always returns successfully._
///
///     The query is parsed by `T`'s `FromQuery` implementation. If the parse
///     succeeds, a `Some(parsed_value)` is returned. Otherwise, a `None` is
///     returned.
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromQuery**
///
///     _This implementation always returns successfully._
///
///     The path segment is parsed by `T`'s `FromQuery` mplementation. The
///     returned `Result` value is returned.
///
/// # Example
///
/// Explicitly implementing `FromQuery` should be rare. For most use-cases, a
/// query guard of `Form<T>` or `LenientForm<T>`, coupled with deriving
/// `FromForm` (as in the previous example) will suffice. For special cases
/// however, an implementation of `FromQuery` may be warranted.
///
/// Consider a contrived scheme where we expect to recieve one query key, `key`,
/// three times and wish to take the middle value. For instance, consider the
/// query:
///
/// ```text
/// key=first_value&key=second_value&key=third_value
/// ```
///
/// We wish to extract `second_value` from this query into a `Contrived` struct.
/// Because `Form` and `LenientForm` will take the _last_ value (`third_value`
/// here) and don't check that there are exactly three keys named `key`, we
/// cannot make use of them and must implement `FromQuery` manually. Such an
/// implementation might look like:
///
/// ```rust
/// use rocket::http::RawStr;
/// use rocket::request::{Query, FromQuery};
///
/// /// Our custom query guard.
/// struct Contrived<'q>(&'q RawStr);
///
/// impl<'q> FromQuery<'q> for Contrived<'q> {
///     /// The number of `key`s we actually saw.
///     type Error = usize;
///
///     fn from_query(query: Query<'q>) -> Result<Self, Self::Error> {
///         let mut key_items = query.filter(|i| i.key == "key");
///
///         // This is cloning an iterator, which is cheap.
///         let count = key_items.clone().count();
///         if count != 3 {
///             return Err(count);
///         }
///
///         // The `ok_or` gets us a `Result`. We will never see `Err(0)`.
///         key_items.map(|i| Contrived(i.value)).nth(1).ok_or(0)
///     }
/// }
/// ```
pub trait FromQuery<'q>: Sized {
    /// The associated error to be returned if parsing/validation fails.
    type Error;

    /// Parses and validates an instance of `Self` from a query or returns an
    /// `Error` if parsing or validation fails.
    fn from_query(query: Query<'q>) -> Result<Self, Self::Error>;
}

impl<'q, T: FromForm<'q>> FromQuery<'q> for Form<T> {
    type Error = T::Error;

    #[inline]
    fn from_query(q: Query<'q>) -> Result<Self, Self::Error> {
        T::from_form(&mut FormItems::from(q.0), true).map(Form)
    }
}

impl<'q, T: FromForm<'q>> FromQuery<'q> for LenientForm<T> {
    type Error = <T as FromForm<'q>>::Error;

    #[inline]
    fn from_query(q: Query<'q>) -> Result<Self, Self::Error> {
        T::from_form(&mut FormItems::from(q.0), false).map(LenientForm)
    }
}

impl<'q, T: FromQuery<'q>> FromQuery<'q> for Option<T> {
    type Error = !;

    #[inline]
    fn from_query(q: Query<'q>) -> Result<Self, Self::Error> {
        Ok(T::from_query(q).ok())
    }
}

impl<'q, T: FromQuery<'q>> FromQuery<'q> for Result<T, T::Error> {
    type Error = !;

    #[inline]
    fn from_query(q: Query<'q>) -> Result<Self, Self::Error> {
        Ok(T::from_query(q))
    }
}
