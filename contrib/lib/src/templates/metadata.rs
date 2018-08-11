use rocket::{Request, State, Outcome};
use rocket::http::Status;
use rocket::request::{self, FromRequest};

use super::ContextManager;

/// The `TemplateMetadata` type: implements `FromRequest`, allowing dynamic
/// queries about template metadata.
///
/// # Usage
///
/// First, ensure that the template [fairing](`rocket::fairing`) is attached to
/// your Rocket application:
///
/// ```rust
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// #
/// use rocket_contrib::Template;
///
/// fn main() {
///     rocket::ignite()
///         .attach(Template::fairing())
///         // ...
///     # ;
/// }
/// ```
///
/// The `TemplateMetadata` type implements Rocket's `FromRequest` trait, so it can
/// be used as a request guard in any request handler.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # #[macro_use] extern crate rocket_contrib;
/// # fn main() {  }
/// #
/// use rocket_contrib::{Template, TemplateMetadata};
///
/// #[get("/")]
/// fn homepage(metadata: TemplateMetadata) -> Template {
///     // Conditionally render a template if it's available.
///     if metadata.contains_template("some-template") {
///         Template::render("some-template", json!({ /* .. */ }))
///     } else {
///         Template::render("fallback", json!({ /* .. */ }))
///     }
/// }
/// ```
pub struct TemplateMetadata<'a>(&'a ContextManager);

impl<'a> TemplateMetadata<'a> {
    /// Returns `true` if the template with name `name` was loaded at start-up
    /// time. Otherwise, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket_contrib::TemplateMetadata;
    ///
    /// fn handler(metadata: TemplateMetadata) {
    ///     // Returns `true` if the template with name `"name"` was loaded.
    ///     let loaded = metadata.contains_template("name");
    /// }
    /// ```
    pub fn contains_template(&self, name: &str) -> bool {
        self.0.context().templates.contains_key(name)
    }
}

/// Retrieves the template metadata. If a template fairing hasn't been attached,
/// an error is printed and an empty `Err` with status `InternalServerError`
/// (`500`) is returned.
impl<'a, 'r> FromRequest<'a, 'r> for TemplateMetadata<'a> {
    type Error = ();

    fn from_request(request: &'a Request) -> request::Outcome<Self, ()> {
        request.guard::<State<ContextManager>>()
            .succeeded()
            .and_then(|cm| Some(Outcome::Success(TemplateMetadata(cm.inner()))))
            .unwrap_or_else(|| {
                error_!("Uninitialized template context: missing fairing.");
                info_!("To use templates, you must attach `Template::fairing()`.");
                info_!("See the `Template` documentation for more information.");
                Outcome::Failure((Status::InternalServerError, ()))
            })
    }
}
