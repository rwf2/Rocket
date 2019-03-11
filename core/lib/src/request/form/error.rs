use std::io;
use http::RawStr;

/// Error returned by the [`FromForm`](::request::FromForm) derive on form
/// parsing errors.
///
/// If multiple errors occur while parsing a form, the first error in the
/// following precedence, from highest to lowest, is returned:
///
///   * `BadValue` or `Unknown` in incoming form string field order
///   * `Missing` in lexical field order
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FormParseError<'f> {
    /// The field named `.0` with value `.1` failed to parse or validate.
    BadValue(&'f RawStr, &'f RawStr),
    /// The parse was strict and the field named `.0` with value `.1` appeared
    /// in the incoming form string but was unexpected.
    ///
    /// This error cannot occur when parsing is lenient.
    Unknown(&'f RawStr, &'f RawStr),
    /// The field named `.0` was expected but is missing in the incoming form.
    Missing(&'f RawStr),
}

/// Error returned by the [`FromData`](::data::FromData) implementations of
/// [`Form`](::request::Form) and [`LenientForm`](::request::LenientForm).
#[derive(Debug)]
pub enum FormDataError<'f, E> {
    /// An I/O error occurred while reading reading the data stream. This can
    /// also mean that the form contained invalid UTF-8.
    Io(io::Error),
    /// The form string (in `.0`) is malformed and was unable to be parsed as
    /// HTTP `application/x-www-form-urlencoded` data.
    Malformed(&'f str),
    /// The form string (in `.1`) failed to parse as the intended structure. The
    /// error type in `.0` contains further details.
    Parse(E, &'f str)
}

/// Alias to the type of form errors returned by the [`FromData`]
/// implementations of [`Form<T>`] where the [`FromForm`] implementation for `T`
/// was derived.
///
/// This alias is particularly useful when "catching" form errors in routes.
///
/// [`FromData`]: ::data::FromData
/// [`Form<T>`]: ::request::Form
/// [`FromForm`]: ::request::FromForm
///
/// # Example
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::request::{Form, FormError, FormDataError};
///
/// #[derive(FromForm)]
/// struct Input {
///     value: String,
/// }
///
/// #[post("/", data = "<sink>")]
/// fn submit(sink: Result<Form<Input>, FormError>) -> String {
///     match sink {
///         Ok(form) => form.into_inner().value,
///         Err(FormDataError::Io(_)) => "I/O error".into(),
///         Err(FormDataError::Malformed(f)) | Err(FormDataError::Parse(_, f)) => {
///             format!("invalid form input: {}", f)
///         }
///     }
/// }
/// # fn main() {}
/// ```
pub type FormError<'f> = FormDataError<'f, FormParseError<'f>>;
