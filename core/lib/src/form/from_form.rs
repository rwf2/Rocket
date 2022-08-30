use std::borrow::Cow;
use std::collections::{HashMap, BTreeMap};
use std::hash::Hash;
use std::sync::Arc;

use either::Either;
use indexmap::IndexMap;

use crate::form::prelude::*;
use crate::http::uncased::AsUncased;

/// Trait implemented by form guards: types parseable from HTTP forms.
///
/// Only form guards that are _collections_, that is, collect more than one form
/// field while parsing, should implement `FromForm`. All other types should
/// implement [`FromFormField`] instead, which offers a simplified interface to
/// parsing a single form field.
///
/// For a gentle introduction to forms in Rocket, see the [forms guide].
///
/// # Form Guards
///
/// A form guard is a guard that operates on form fields, typically those with a
/// particular name prefix. Form guards validate and parse form field data via
/// implementations of `FromForm`. In other words, a type is a form guard _iff_
/// it implements `FromFrom`.
///
/// Form guards are used as the inner type of the [`Form`] data guard:
///
/// ```rust
/// # use rocket::post;
/// use rocket::form::Form;
///
/// # type FormGuard = String;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: Form<FormGuard>) { /* ... */ }
/// ```
///
/// # Deriving
///
/// This trait can, and largely _should_, be automatically derived. When
/// deriving `FromForm`, every field in the structure must implement
/// [`FromForm`]. Form fields with the struct field's name are [shifted] and
/// then pushed to the struct field's `FromForm` parser.
///
/// [shifted]: NameView::shift()
///
/// ```rust
/// use rocket::form::FromForm;
///
/// #[derive(FromForm)]
/// struct TodoTask<'r> {
///     #[field(validate = len(1..))]
///     description: &'r str,
///     #[field(name = "done")]
///     completed: bool
/// }
/// ```
///
/// For full details on deriving `FromForm`, see the [`FromForm` derive].
///
/// [`Form`]: crate::form::Form
/// [`FromForm`]: crate::form::FromForm
/// [`FromForm` derive]: derive@crate::FromForm
/// [FromFormField]: crate::form::FromFormField
/// [`shift()`ed]: NameView::shift()
/// [`key()`]: NameView::key()
/// [forms guide]: https://rocket.rs/v0.5-rc/guide/requests/#forms
///
/// # Parsing Strategy
///
/// Form parsing is either _strict_ or _lenient_, controlled by
/// [`Options::strict`]. A _strict_ parse errors when there are missing or extra
/// fields, while a _lenient_ parse allows both, providing there is a
/// [`default()`](FromForm::default()) in the case of a missing field.
///
/// Most type inherit their strategy on [`FromForm::init()`], but some types
/// like `Option` override the requested strategy. The strategy can also be
/// overwritten manually, per-field or per-value, by using the [`Strict`] or
/// [`Lenient`] form guard:
///
/// ```rust
/// use rocket::form::{self, FromForm, Strict, Lenient};
///
/// #[derive(FromForm)]
/// struct TodoTask<'r> {
///     strict_bool: Strict<bool>,
///     lenient_inner_option: Option<Lenient<bool>>,
///     strict_inner_result: form::Result<'r, Strict<bool>>,
/// }
/// ```
///
/// # Defaults
///
/// A form guard may have a _default_ which is used in case of a missing field
/// when parsing is _lenient_. When parsing is strict, all errors, including
/// missing fields, are propagated directly.
///
/// # Provided Implementations
///
/// Rocket implements `FromForm` for many common types. As a result, most
/// applications will never need a custom implementation of `FromForm` or
/// `FromFormField`. Their behavior is documented in the table below.
///
/// | Type               | Strategy    | Default           | Data   | Value  | Notes                                              |
/// |--------------------|-------------|-------------------|--------|--------|----------------------------------------------------|
/// | [`Strict<T>`]      | **strict**  | if `strict` `T`   | if `T` | if `T` | `T: FromForm`                                      |
/// | [`Lenient<T>`]     | **lenient** | if `lenient` `T`  | if `T` | if `T` | `T: FromForm`                                      |
/// | `Option<T>`        | **strict**  | `None`            | if `T` | if `T` | Infallible, `T: FromForm`                          |
/// | [`Result<T>`]      | _inherit_   | `T::finalize()`   | if `T` | if `T` | Infallible, `T: FromForm`                          |
/// | `Vec<T>`           | _inherit_   | `vec![]`          | if `T` | if `T` | `T: FromForm`                                      |
/// | [`HashMap<K, V>`]  | _inherit_   | `HashMap::new()`  | if `V` | if `V` | `K: FromForm + Eq + Hash`, `V: FromForm`           |
/// | [`BTreeMap<K, V>`] | _inherit_   | `BTreeMap::new()` | if `V` | if `V` | `K: FromForm + Ord`, `V: FromForm`                 |
/// | `bool`             | _inherit_   | `false`           | No     | Yes    | `"yes"/"on"/"true"`, `"no"/"off"/"false"`          |
/// | (un)signed int     | _inherit_   | **no default**    | No     | Yes    | `{u,i}{size,8,16,32,64,128}`                       |
/// | _nonzero_ int      | _inherit_   | **no default**    | No     | Yes    | `NonZero{I,U}{size,8,16,32,64,128}`                |
/// | float              | _inherit_   | **no default**    | No     | Yes    | `f{32,64}`                                         |
/// | `&str`             | _inherit_   | **no default**    | Yes    | Yes    | Percent-decoded. Data limit `string` applies.      |
/// | `String`           | _inherit_   | **no default**    | Yes    | Yes    | Exactly `&str`, but owned. Prefer `&str`.          |
/// | IP Address         | _inherit_   | **no default**    | No     | Yes    | [`IpAddr`], [`Ipv4Addr`], [`Ipv6Addr`]             |
/// | Socket Address     | _inherit_   | **no default**    | No     | Yes    | [`SocketAddr`], [`SocketAddrV4`], [`SocketAddrV6`] |
/// | [`TempFile`]       | _inherit_   | **no default**    | Yes    | Yes    | Data limits apply. See [`TempFile`].               |
/// | [`Capped<C>`]      | _inherit_   | **no default**    | Yes    | Yes    | `C` is `&str`, `String`, or `TempFile`.            |
/// | [`time::Date`]     | _inherit_   | **no default**    | No     | Yes    | `%F` (`YYYY-MM-DD`). HTML "date" input.            |
/// | [`time::DateTime`] | _inherit_   | **no default**    | No     | Yes    | `%FT%R` or `%FT%T` (`YYYY-MM-DDTHH:MM[:SS]`)       |
/// | [`time::Time`]     | _inherit_   | **no default**    | No     | Yes    | `%R` or `%T` (`HH:MM[:SS]`)                        |
///
/// [`Result<T>`]: crate::form::Result
/// [`Strict<T>`]: crate::form::Strict
/// [`Lenient<T>`]: crate::form::Lenient
/// [`HashMap<K, V>`]: std::collections::HashMap
/// [`BTreeMap<K, V>`]: std::collections::BTreeMap
/// [`TempFile`]: crate::fs::TempFile
/// [`Capped<C>`]: crate::data::Capped
/// [`time::DateTime`]: time::PrimitiveDateTime
/// [`IpAddr`]: std::net::IpAddr
/// [`Ipv4Addr`]: std::net::Ipv4Addr
/// [`Ipv6Addr`]: std::net::Ipv6Addr
/// [`SocketAddr`]: std::net::SocketAddr
/// [`SocketAddrV4`]: std::net::SocketAddrV4
/// [`SocketAddrV6`]: std::net::SocketAddrV6
///
/// ## Additional Notes
///
///   * **`Vec<T>` where `T: FromForm`**
///
///     Parses a sequence of `T`'s. A new `T` is created whenever the field
///     name's key changes or is empty; the previous `T` is finalized and errors
///     are stored. While the key remains the same and non-empty, form values
///     are pushed to the current `T` after being shifted. All collected errors
///     are returned at finalization, if any, or the successfully created vector
///     is returned.
///
///   * **`HashMap<K, V>` where `K: FromForm + Eq + Hash`, `V: FromForm`**
///
///     **`BTreeMap<K, V>` where `K: FromForm + Ord`, `V: FromForm`**
///
///     Parses a sequence of `(K, V)`'s. A new pair is created for every unique
///     first index of the key.
///
///     If the key has only one index (`map[index]=value`), the index itself is
///     pushed to `K`'s parser and the remaining shifted field is pushed to
///     `V`'s parser.
///
///     If the key has two indices (`map[k:index]=value` or
///     `map[v:index]=value`), the first index must start with `k` or `v`. If
///     the first index starts with `k`, the shifted field is pushed to `K`'s
///     parser. If the first index starts with `v`, the shifted field is pushed
///     to `V`'s parser. If the first index is anything else, an error is
///     created for the offending form field.
///
///     Errors are collected as they occur. Finalization finalizes all pairs and
///     returns errors, if any, or the map.
///
///   * **`bool`**
///
///     Parses as `false` for missing values (when lenient) and case-insensitive
///     values of `off`, `false`, and `no`. Parses as `true` for values of `on`,
///     `true`, `yes`, and the empty value. Failed to parse otherwise.
///
///   * **[`time::DateTime`]**
///
///     Parses a date in `%FT%R` or `%FT%T` format, that is, `YYYY-MM-DDTHH:MM`
///     or `YYYY-MM-DDTHH:MM:SS`. This is the `"datetime-local"` HTML input type
///     without support for the millisecond variant.
///
///   * **[`time::Time`]**
///
///     Parses a time in `%R` or `%T` format, that is, `HH:MM` or `HH:MM:SS`.
///     This is the `"time"` HTML input type without support for the millisecond
///     variant.
///
/// # Push Parsing
///
/// `FromForm` describes a push-based parser for Rocket's [field wire format].
/// Fields are preprocessed into either [`ValueField`]s or [`DataField`]s which
/// are then pushed to the parser in [`FromForm::push_value()`] or
/// [`FromForm::push_data()`], respectively. Both url-encoded forms and
/// multipart forms are supported. All url-encoded form fields are preprocessed
/// as [`ValueField`]s. Multipart form fields with Content-Types are processed
/// as [`DataField`]s while those without a set Content-Type are processed as
/// [`ValueField`]s. `ValueField` field names and values are percent-decoded.
///
/// [field wire format]: crate::form#field-wire-format
///
/// Parsing is split into 3 stages. After preprocessing, the three stages are:
///
///   1. **Initialization.** The type sets up a context for later `push`es.
///
///      ```rust
///      # use rocket::form::prelude::*;
///      # struct Foo;
///      use rocket::form::Options;
///
///      # #[rocket::async_trait]
///      # impl<'r> FromForm<'r> for Foo {
///          # type Context = std::convert::Infallible;
///      fn init(opts: Options) -> Self::Context {
///          todo!("return a context for storing parse state")
///      }
///          # fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) { todo!() }
///          # async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) { todo!() }
///          # fn finalize(ctxt: Self::Context) -> Result<'r, Self> { todo!() }
///      # }
///      ```
///
///   2. **Push.** The structure is repeatedly pushed form fields; the latest
///      context is provided with each `push`. If the structure contains
///      children, it uses the first [`key()`] to identify a child to which it
///      then `push`es the remaining `field` to, likely with a [`shift()`ed]
///      name. Otherwise, the structure parses the `value` itself. The context
///      is updated as needed.
///
///      ```rust
///      # use rocket::form::prelude::*;
///      # struct Foo;
///      use rocket::form::{ValueField, DataField};
///
///      # #[rocket::async_trait]
///      # impl<'r> FromForm<'r> for Foo {
///          # type Context = std::convert::Infallible;
///          # fn init(opts: Options) -> Self::Context { todo!() }
///      fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
///          todo!("modify context as necessary for `field`")
///      }
///
///      async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
///          todo!("modify context as necessary for `field`")
///      }
///          # fn finalize(ctxt: Self::Context) -> Result<'r, Self> { todo!() }
///      # }
///      ```
///
///   3. **Finalization.** The structure is informed that there are no further
///      fields. It systemizes the effects of previous `push`es via its context
///      to return a parsed structure or generate [`Errors`].
///
///      ```rust
///      # use rocket::form::prelude::*;
///      # struct Foo;
///      use rocket::form::Result;
///
///      # #[rocket::async_trait]
///      # impl<'r> FromForm<'r> for Foo {
///          # type Context = std::convert::Infallible;
///          # fn init(opts: Options) -> Self::Context { todo!() }
///          # fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) { todo!() }
///          # async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) { todo!() }
///      fn finalize(ctxt: Self::Context) -> Result<'r, Self> {
///          todo!("inspect context to generate `Self` or `Errors`")
///      }
///      # }
///      ```
///
/// These three stages make up the entirety of the `FromForm` trait.
///
/// ## Nesting and [`NameView`]
///
/// Each field name key typically identifies a unique child of a structure. As
/// such, when processed left-to-right, the keys of a field jointly identify a
/// unique leaf of a structure. The value of the field typically represents the
/// desired value of the leaf.
///
/// A [`NameView`] captures and simplifies this "left-to-right" processing of a
/// field's name by exposing a sliding-prefix view into a name. A [`shift()`]
/// shifts the view one key to the right. Thus, a `Name` of `a.b.c` when viewed
/// through a new [`NameView`] is `a`. Shifted once, the view is `a.b`.
/// [`key()`] returns the last (or "current") key in the view. A nested
/// structure can thus handle a field with a `NameView`, operate on the
/// [`key()`], [`shift()`] the `NameView`, and pass the field with the shifted
/// `NameView` to the next processor which handles `b` and so on.
///
/// [`shift()`]: NameView::shift()
/// [`key()`]: NameView::key()
///
/// ## A Simple Example
///
/// The following example uses `f1=v1&f2=v2` to illustrate field/value pairs
/// `(f1, v2)` and `(f2, v2)`. This is the same encoding used to send HTML forms
/// over HTTP, though Rocket's push-parsers are unaware of any specific
/// encoding, dealing only with logical `field`s, `index`es, and `value`s.
///
/// ### A Single Field (`T: FormFormField`)
///
/// The simplest example parses a single value of type `T` from a string with an
/// optional default value: this is `impl<T: FromFormField> FromForm for T`:
///
///   1. **Initialization.** The context stores form options and an `Option` of
///      `Result<T, form::Error>` for storing the `result` of parsing `T`, which
///      is initially set to `None`.
///
///      ```rust
///      use rocket::form::{self, FromFormField};
///
///      struct Context<'r, T: FromFormField<'r>> {
///          opts: form::Options,
///          result: Option<form::Result<'r, T>>,
///      }
///
///      # impl<'r, T: FromFormField<'r>> Context<'r, T> {
///      fn init(opts: form::Options) -> Context<'r, T> {
///         Context { opts, result: None }
///      }
///      # }
///      ```
///
///   2. **Push.** If `ctxt.result` is `None`, `T` is parsed from `field`, and
///      the result is stored in `context.result`. Otherwise a field has already
///      been parsed and nothing is done.
///
///      ```rust
///      # use rocket::form::{self, ValueField, FromFormField};
///      # struct Context<'r, T: FromFormField<'r>> {
///      #     opts: form::Options,
///      #     result: Option<form::Result<'r, T>>,
///      # }
///      # impl<'r, T: FromFormField<'r>> Context<'r, T> {
///      fn push_value(ctxt: &mut Context<'r, T>, field: ValueField<'r>) {
///          if ctxt.result.is_none() {
///              ctxt.result = Some(T::from_value(field));
///          }
///      }
///      # }
///      ```
///
///   3. **Finalization.** If `ctxt.result` is `None`, parsing is lenient, and
///      `T` has a default, the default is returned. Otherwise a `Missing` error
///      is returned. If `ctxt.result` is `Some(v)`, the result `v` is returned.
///
///      ```rust
///      # use rocket::form::{self, FromFormField, error::{Errors, ErrorKind}};
///      # struct Context<'r, T: FromFormField<'r>> {
///      #     opts: form::Options,
///      #     result: Option<form::Result<'r, T>>,
///      # }
///      # impl<'r, T: FromFormField<'r>> Context<'r, T> {
///      fn finalize(ctxt: Context<'r, T>) -> form::Result<'r, T> {
///          match ctxt.result {
///              Some(result) => result,
///              None if ctxt.opts.strict => Err(Errors::from(ErrorKind::Missing)),
///              None => match T::default() {
///                  Some(default) => Ok(default),
///                  None => Err(Errors::from(ErrorKind::Missing)),
///              }
///          }
///      }
///      # }
///      ```
///
/// This implementation is complete except for the following details:
///
///   * handling both `push_data` and `push_value`
///   * checking for duplicate pushes when parsing is `strict`
///   * tracking the field's name and value to generate a complete [`Error`]
///
/// # Implementing
///
/// Implementing `FromForm` should be a rare occurrence. Prefer instead to use
/// Rocket's built-in derivation or, for custom types, implementing
/// [`FromFormField`].
///
/// An implementation of `FromForm` consists of implementing the three stages
/// outlined above. `FromForm` is an async trait, so implementations must be
/// decorated with an attribute of `#[rocket::async_trait]`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # struct MyType;
/// # struct MyContext;
/// use rocket::form::{self, FromForm, DataField, ValueField};
///
/// #[rocket::async_trait]
/// impl<'r> FromForm<'r> for MyType {
///     type Context = MyContext;
///
///     fn init(opts: form::Options) -> Self::Context {
///         todo!()
///     }
///
///     fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
///         todo!()
///     }
///
///     async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
///         todo!()
///     }
///
///     fn finalize(this: Self::Context) -> form::Result<'r, Self> {
///         todo!()
///     }
/// }
/// ```
///
/// The lifetime `'r` correponds to the lifetime of the request.
///
/// ## A More Involved Example
///
/// We illustrate implementation of `FromForm` through an example. The example
/// implements `FromForm` for a `Pair(A, B)` type where `A: FromForm` and `B:
/// FromForm`, parseable from forms with at least two fields, one with a key of
/// `0` and the other with a key of `1`. The field with key `0` is parsed as an
/// `A` while the field with key `1` is parsed as a `B`. Specifically, to parse
/// a `Pair(A, B)` from a field with prefix `pair`, a form with the following
/// fields must be submitted:
///
///   * `pair[0]` - type A
///   * `pair[1]` - type B
///
/// Examples include:
///
///   * `pair[0]=id&pair[1]=100` as `Pair(&str, usize)`
///   * `pair[0]=id&pair[1]=100` as `Pair(&str, &str)`
///   * `pair[0]=2012-10-12&pair[1]=100` as `Pair(time::Date, &str)`
///   * `pair.0=2012-10-12&pair.1=100` as `Pair(time::Date, usize)`
///
/// ```rust
/// use either::Either;
/// use rocket::form::{self, FromForm, ValueField, DataField, Error, Errors};
///
/// /// A form guard parseable from fields `.0` and `.1`.
/// struct Pair<A, B>(A, B);
///
/// // The parsing context. We'll be pushing fields with key `.0` to `left`
/// // and fields with `.1` to `right`. We'll collect errors along the way.
/// struct PairContext<'v, A: FromForm<'v>, B: FromForm<'v>> {
///     left: A::Context,
///     right: B::Context,
///     errors: Errors<'v>,
/// }
///
/// #[rocket::async_trait]
/// impl<'v, A: FromForm<'v>, B: FromForm<'v>> FromForm<'v> for Pair<A, B> {
///     type Context = PairContext<'v, A, B>;
///
///     // We initialize the `PairContext` as expected.
///     fn init(opts: form::Options) -> Self::Context {
///         PairContext {
///             left: A::init(opts),
///             right: B::init(opts),
///             errors: Errors::new()
///         }
///     }
///
///     // For each value, we determine if the key is `.0` (left) or `.1`
///     // (right) and push to the appropriate parser. If it was neither, we
///     // store the error for emission on finalization. The parsers for `A` and
///     // `B` will handle duplicate values and so on.
///     fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
///         match ctxt.context(field.name) {
///             Ok(Either::Left(ctxt)) => A::push_value(ctxt, field.shift()),
///             Ok(Either::Right(ctxt)) => B::push_value(ctxt, field.shift()),
///             Err(e) => ctxt.errors.push(e),
///         }
///     }
///
///     // This is identical to `push_value` but for data fields.
///     async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
///         match ctxt.context(field.name) {
///             Ok(Either::Left(ctxt)) => A::push_data(ctxt, field.shift()).await,
///             Ok(Either::Right(ctxt)) => B::push_data(ctxt, field.shift()).await,
///             Err(e) => ctxt.errors.push(e),
///         }
///     }
///
///     // Finally, we finalize `A` and `B`. If both returned `Ok` and we
///     // encountered no errors during the push phase, we return our pair. If
///     // there were errors, we return them. If `A` and/or `B` failed, we
///     // return the commulative errors.
///     fn finalize(mut ctxt: Self::Context) -> form::Result<'v, Self> {
///         match (A::finalize(ctxt.left), B::finalize(ctxt.right)) {
///             (Ok(l), Ok(r)) if ctxt.errors.is_empty() => Ok(Pair(l, r)),
///             (Ok(_), Ok(_)) => Err(ctxt.errors),
///             (left, right) => {
///                 if let Err(e) = left { ctxt.errors.extend(e); }
///                 if let Err(e) = right { ctxt.errors.extend(e); }
///                 Err(ctxt.errors)
///             }
///         }
///     }
/// }
///
/// impl<'v, A: FromForm<'v>, B: FromForm<'v>> PairContext<'v, A, B> {
///     // Helper method used by `push_{value, data}`. Determines which context
///     // we should push to based on the field name's key. If the key is
///     // neither `0` nor `1`, we return an error.
///     fn context(
///         &mut self,
///         name: form::name::NameView<'v>
///     ) -> Result<Either<&mut A::Context, &mut B::Context>, Error<'v>> {
///         use std::borrow::Cow;
///
///         match name.key().map(|k| k.as_str()) {
///             Some("0") => Ok(Either::Left(&mut self.left)),
///             Some("1") => Ok(Either::Right(&mut self.right)),
///             _ => Err(Error::from(&[Cow::Borrowed("0"), Cow::Borrowed("1")])
///                 .with_entity(form::error::Entity::Index(0))
///                 .with_name(name)),
///         }
///     }
/// }
/// ```
#[crate::async_trait]
pub trait FromForm<'r>: Send + Sized {
    /// The form guard's parsing context.
    type Context: Send;

    /// Initializes and returns the parsing context for `Self`.
    fn init(opts: Options) -> Self::Context;

    /// Processes the value field `field`.
    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>);

    /// Processes the data field `field`.
    async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>);

    /// Processes the external form or field error `_error`.
    ///
    /// The default implementation does nothing, which is always correct.
    fn push_error(_ctxt: &mut Self::Context, _error: Error<'r>) { }

    /// Finalizes parsing. Returns the parsed value when successful or
    /// collection of [`Errors`] otherwise.
    fn finalize(ctxt: Self::Context) -> Result<'r, Self>;

    /// Returns a default value, if any, to use when a value is desired and
    /// parsing fails.
    ///
    /// The default implementation initializes `Self` with `opts` and finalizes
    /// immediately, returning the value if finalization succeeds. This is
    /// always correct and should likely not be changed. Returning a different
    /// value may result in ambiguous parses.
    fn default(opts: Options) -> Option<Self> {
        Self::finalize(Self::init(opts)).ok()
    }
}

#[doc(hidden)]
pub struct VecContext<'v, T: FromForm<'v>> {
    opts: Options,
    last_key: Option<&'v Key>,
    current: Option<T::Context>,
    errors: Errors<'v>,
    items: Vec<T>
}

impl<'v, T: FromForm<'v>> VecContext<'v, T> {
    fn new(opts: Options) -> Self {
        VecContext {
            opts,
            last_key: None,
            current: None,
            items: vec![],
            errors: Errors::new(),
        }
    }

    fn shift(&mut self) {
        if let Some(current) = self.current.take() {
            match T::finalize(current) {
                Ok(v) => self.items.push(v),
                Err(e) => self.errors.extend(e)
            }
        }
    }

    fn context(&mut self, name: &NameView<'v>) -> &mut T::Context {
        let this_key = name.key();
        let keys_match = match (self.last_key, this_key) {
            (Some(k1), Some(k2)) => k1 == k2,
            _ => false
        };

        if !keys_match {
            self.shift();
            self.current = Some(T::init(self.opts));
        }

        self.last_key = name.key();
        self.current.as_mut().expect("must have current if last == index")
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v> + 'v> FromForm<'v> for Vec<T> {
    type Context = VecContext<'v, T>;

    fn init(opts: Options) -> Self::Context {
        VecContext::new(opts)
    }

    fn push_value(this: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(this.context(&field.name), field.shift());
    }

    async fn push_data(this: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(this.context(&field.name), field.shift()).await
    }

    fn finalize(mut this: Self::Context) -> Result<'v, Self> {
        this.shift();
        if !this.errors.is_empty() {
            Err(this.errors)
        } else if this.opts.strict && this.items.is_empty() {
            Err(Errors::from(ErrorKind::Missing))
        } else {
            Ok(this.items)
        }
    }
}

#[doc(hidden)]
pub struct MapContext<'v, K, V> where K: FromForm<'v>, V: FromForm<'v> {
    opts: Options,
    /// Maps an index key (&str, map.key=foo, map.k:key) to its entry.
    /// NOTE: `table`, `entries`, and `metadata` are always the same size.
    table: IndexMap<&'v str, usize>,
    /// The `FromForm` context for the (key, value) indexed by `table`.
    entries: Vec<(K::Context, V::Context)>,
    /// Recorded metadata for a given key/value pair.
    metadata: Vec<NameView<'v>>,
    /// Errors collected while finalizing keys and values.
    errors: Errors<'v>,
}

impl<'v, K, V> MapContext<'v, K, V>
    where K: FromForm<'v>, V: FromForm<'v>
{
    fn new(opts: Options) -> Self {
        MapContext {
            opts,
            table: IndexMap::new(),
            entries: vec![],
            metadata: vec![],
            errors: Errors::new(),
        }
    }

    fn ctxt(&mut self, key: &'v str, name: NameView<'v>) -> &mut (K::Context, V::Context) {
        match self.table.get(key) {
            Some(i) => &mut self.entries[*i],
            None => {
                let i = self.entries.len();
                self.table.insert(key, i);
                self.entries.push((K::init(self.opts), V::init(self.opts)));
                self.metadata.push(name);
                &mut self.entries[i]
            }
        }
    }

    fn push(&mut self, name: NameView<'v>) -> Option<Either<&mut K::Context, &mut V::Context>> {
        let index_pair = name.key()
            .map(|k| k.indices())
            .map(|mut i| (i.next(), i.next()))
            .unwrap_or_default();

        match index_pair {
            (Some(key), None) => {
                let is_new_key = !self.table.contains_key(key);
                let (key_ctxt, val_ctxt) = self.ctxt(key, name);
                if is_new_key {
                    K::push_value(key_ctxt, ValueField::from_value(key));
                }

                return Some(Either::Right(val_ctxt));
            },
            (Some(kind), Some(key)) => {
                if kind.as_uncased().starts_with("k") {
                    return Some(Either::Left(&mut self.ctxt(key, name).0));
                } else if kind.as_uncased().starts_with("v") {
                    return Some(Either::Right(&mut self.ctxt(key, name).1));
                } else {
                    let error = Error::from(&[Cow::Borrowed("k"), Cow::Borrowed("v")])
                        .with_entity(Entity::Index(0))
                        .with_name(name);

                    self.errors.push(error);
                }
            }
            _ => {
                let error = Error::from(ErrorKind::Missing)
                    .with_entity(Entity::Key)
                    .with_name(name);

                self.errors.push(error);
            }
        };

        None
    }

    fn push_value(&mut self, field: ValueField<'v>) {
        match self.push(field.name) {
            Some(Either::Left(ctxt)) => K::push_value(ctxt, field.shift()),
            Some(Either::Right(ctxt)) => V::push_value(ctxt, field.shift()),
            _ => {}
        }
    }

    async fn push_data(&mut self, field: DataField<'v, '_>) {
        match self.push(field.name) {
            Some(Either::Left(ctxt)) => K::push_data(ctxt, field.shift()).await,
            Some(Either::Right(ctxt)) => V::push_data(ctxt, field.shift()).await,
            _ => {}
        }
    }

    fn finalize<T: std::iter::FromIterator<(K, V)>>(mut self) -> Result<'v, T> {
        let map: T = self.entries.into_iter()
            .zip(self.metadata.iter())
            .zip(self.table.keys())
            .filter_map(|(((k_ctxt, v_ctxt), name), idx)| {
                let key = K::finalize(k_ctxt)
                    .map_err(|e| {
                        // FIXME: Fix `NameBuf` to take in `k` and add it.
                        // FIXME: Perhaps the `k` should come after: `map.0:k`.
                        let form_key = format!("k:{}", idx);
                        self.errors.extend(e.with_name((name.parent(), form_key)));
                    })
                    .ok();

                let val = V::finalize(v_ctxt)
                    .map_err(|e| self.errors.extend(e.with_name((name.parent(), *idx))))
                    .ok();

                Some((key?, val?))
            })
            .collect();

        if !self.errors.is_empty() {
            Err(self.errors)
        } else if self.opts.strict && self.table.is_empty() {
            Err(Errors::from(ErrorKind::Missing))
        } else {
            Ok(map)
        }
    }
}

#[crate::async_trait]
impl<'v, K, V> FromForm<'v> for HashMap<K, V>
    where K: FromForm<'v> + Eq + Hash, V: FromForm<'v>
{
    type Context = MapContext<'v, K, V>;

    fn init(opts: Options) -> Self::Context {
        MapContext::new(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        ctxt.push_value(field);
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        ctxt.push_data(field).await;
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        this.finalize()
    }
}

#[crate::async_trait]
impl<'v, K, V> FromForm<'v> for BTreeMap<K, V>
    where K: FromForm<'v> + Ord, V: FromForm<'v>
{
    type Context = MapContext<'v, K, V>;

    fn init(opts: Options) -> Self::Context {
        MapContext::new(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        ctxt.push_value(field);
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        ctxt.push_data(field).await;
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        this.finalize()
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Option<T> {
    type Context = <T as FromForm<'v>>::Context;

    fn init(opts: Options) -> Self::Context {
        T::init(Options { strict: true, ..opts })
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        Ok(T::finalize(this).ok())
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Result<'v, T> {
    type Context = <T as FromForm<'v>>::Context;

    fn init(opts: Options) -> Self::Context {
        T::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        Ok(T::finalize(this))
    }
}

#[doc(hidden)]
pub struct PairContext<'v, A: FromForm<'v>, B: FromForm<'v>> {
    left: A::Context,
    right: B::Context,
    errors: Errors<'v>,
}

impl<'v, A: FromForm<'v>, B: FromForm<'v>> PairContext<'v, A, B> {
    fn context(
        &mut self,
        name: NameView<'v>
    ) -> std::result::Result<Either<&mut A::Context, &mut B::Context>, Error<'v>> {
        match name.key().map(|k| k.as_str()) {
            Some("0") => Ok(Either::Left(&mut self.left)),
            Some("1") => Ok(Either::Right(&mut self.right)),
            _ => Err(Error::from(&[Cow::Borrowed("0"), Cow::Borrowed("1")])
                .with_entity(Entity::Index(0))
                .with_name(name)),
        }
    }
}

#[crate::async_trait]
impl<'v, A: FromForm<'v>, B: FromForm<'v>> FromForm<'v> for (A, B) {
    type Context = PairContext<'v, A, B>;

    fn init(opts: Options) -> Self::Context {
        PairContext {
            left: A::init(opts),
            right: B::init(opts),
            errors: Errors::new()
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        match ctxt.context(field.name) {
            Ok(Either::Left(ctxt)) => A::push_value(ctxt, field.shift()),
            Ok(Either::Right(ctxt)) => B::push_value(ctxt, field.shift()),
            Err(e) => ctxt.errors.push(e),
        }
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        match ctxt.context(field.name) {
            Ok(Either::Left(ctxt)) => A::push_data(ctxt, field.shift()).await,
            Ok(Either::Right(ctxt)) => B::push_data(ctxt, field.shift()).await,
            Err(e) => ctxt.errors.push(e),
        }
    }

    fn finalize(mut ctxt: Self::Context) -> Result<'v, Self> {
        match (A::finalize(ctxt.left), B::finalize(ctxt.right)) {
            (Ok(key), Ok(val)) if ctxt.errors.is_empty() => Ok((key, val)),
            (Ok(_), Ok(_)) => Err(ctxt.errors)?,
            (left, right) => {
                if let Err(e) = left { ctxt.errors.extend(e); }
                if let Err(e) = right { ctxt.errors.extend(e); }
                Err(ctxt.errors)?
            }
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v> + Sync> FromForm<'v> for Arc<T> {
    type Context = <T as FromForm<'v>>::Context;

    fn init(opts: Options) -> Self::Context {
        T::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        T::finalize(this).map(Arc::new)
    }
}
