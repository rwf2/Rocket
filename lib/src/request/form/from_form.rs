use request::FormItems;

/// Trait to create an instance of some type from an HTTP form.
/// [Form](struct.Form.html) requires its generic type to implement this trait.
///
/// This trait can be automatically derived via the
/// [rocket_codegen](/rocket_codegen) plugin:
///
/// ```rust
/// #![feature(plugin, custom_derive)]
/// #![plugin(rocket_codegen)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
///
/// extern crate rocket;
///
/// #[derive(FromForm)]
/// struct TodoTask {
///     description: String,
///     completed: bool
/// }
/// ```
///
/// The type can then be parsed from incoming form data via the `data`
/// parameter and `Form` type.
///
/// ```rust
/// # #![feature(plugin, custom_derive)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # use rocket::request::Form;
/// # #[derive(FromForm)]
/// # struct TodoTask { description: String, completed: bool }
/// #[post("/submit", data = "<task>")]
/// fn submit_task(task: Form<TodoTask>) -> String {
///     format!("New task: {}", task.get().description)
/// }
/// # fn main() {  }
/// ```
///
/// When deriving `FromForm`, every field in the structure must implement
/// [FromFormValue](trait.FromFormValue.html).
///
/// # Implementing
///
/// An implementation of `FromForm` uses the [FormItems](struct.FormItems.html)
/// iterator to iterate through the raw form key/value pairs. Be aware that form
/// fields that are typically hidden from your application, such as `_method`,
/// will be present while iterating.
pub trait FromForm<'f>: Sized {
    /// The associated error to be returned when parsing fails.
    type Error;

    /// Parses an instance of `Self` from the form items or returns an `Error`
    /// if one cannot be parsed.
    fn from_form_items(form_items: &mut FormItems<'f>) -> Result<Self, Self::Error>;
}

/// This implementation should only be used during debugging!
impl<'f> FromForm<'f> for &'f str {
    type Error = ();

    fn from_form_items(items: &mut FormItems<'f>) -> Result<Self, Self::Error> {
        items.mark_complete();
        Ok(items.inner_str())
    }
}
