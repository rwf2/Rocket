use request::FormItems;

/// Trait to create an instance of some type from an HTTP form.
/// [`Form`](::request::Form) requires its generic type to implement this trait.
///
/// # Deriving
///
/// This trait can be automatically derived. When deriving `FromForm`, every
/// field in the structure must implement
/// [`FromFormValue`](::request::FromFormValue). Rocket validates each field in
/// the structure by calling its `FromFormValue` implementation. You may wish to
/// implement `FromFormValue` for your own types for custom, automatic
/// validation.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
/// # #[macro_use] extern crate rocket;
/// #[derive(FromForm)]
/// struct TodoTask {
///     description: String,
///     completed: bool
/// }
/// # fn main() {  }
/// ```
///
/// # Data Guard
///
/// Types that implement `FromForm` can be parsed directly from incoming form
/// data via the `data` parameter and `Form` type.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
/// # #[macro_use] extern crate rocket;
/// # use rocket::request::Form;
/// # #[derive(FromForm)]
/// # struct TodoTask { description: String, completed: bool }
/// #[post("/submit", data = "<task>")]
/// fn submit_task(task: Form<TodoTask>) -> String {
///     format!("New task: {}", task.description)
/// }
/// # fn main() {  }
/// ```
///
/// # Implementing
///
/// Implementing `FromForm` should be a rare occurrence. Prefer instead to use
/// Rocket's built-in derivation.
///
/// When implementing `FromForm`, use the [`FormItems`] iterator to iterate
/// through the raw form key/value pairs. Be aware that form fields that are
/// typically hidden from your application, such as `_method`, will be present
/// while iterating. Ensure that you adhere to the properties of the `strict`
/// parameter, as detailed in the documentation below.
///
/// ## Example
///
/// Consider the following scenario: we have a struct `Item` with field name
/// `field`. We'd like to parse any form that has a field named either `balloon`
/// _or_ `space`, and we'd like that field's value to be the value for our
/// structure's `field`. The following snippet shows how this would be
/// implemented:
///
/// ```rust
/// use rocket::request::{FromForm, FormItems};
///
/// struct Item {
///     field: String
/// }
///
/// impl<'f> FromForm<'f> for Item {
///     // In practice, we'd use a more descriptive error type.
///     type Error = ();
///
///     fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<Item, ()> {
///         let mut field = None;
///
///         for item in items {
///             match item.key.as_str() {
///                 "balloon" | "space" if field.is_none() => {
///                     let decoded = item.value.url_decode().map_err(|_| ())?;
///                     field = Some(decoded);
///                 }
///                 _ if strict => return Err(()),
///                 _ => { /* allow extra value when not strict */ }
///             }
///         }
///
///         field.map(|field| Item { field }).ok_or(())
///     }
/// }
/// ```
pub trait FromForm<'f>: Sized {
    /// The associated error to be returned when parsing fails.
    type Error;

    /// Parses an instance of `Self` from the iterator of form items `it`.
    ///
    /// Extra form field are allowed when `strict` is `false` and disallowed
    /// when `strict` is `true`.
    ///
    /// # Errors
    ///
    /// If `Self` cannot be parsed from the given form items, an instance of
    /// `Self::Error` will be returned.
    ///
    /// When `strict` is `true` and unexpected, extra fields are present in
    /// `it`, an instance of `Self::Error` will be returned.
    fn from_form(it: &mut FormItems<'f>, strict: bool) -> Result<Self, Self::Error>;
}

impl<'f, T: FromForm<'f>> FromForm<'f> for Option<T> {
    type Error = !;

    #[inline]
    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<Option<T>, !> {
        Ok(T::from_form(items, strict).ok())
    }
}

impl<'f, T: FromForm<'f>> FromForm<'f> for Result<T, T::Error> {
    type Error = !;

    #[inline]
    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<Self, !> {
        Ok(T::from_form(items, strict))
    }
}
