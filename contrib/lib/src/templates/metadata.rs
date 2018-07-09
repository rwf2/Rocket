use rocket::{http::Status, outcome, request::{FromRequest, Outcome}, Request, State};
use templates::context::Context;

/// The TemplateEngine type: implements `FromRequest`, allowing you to communicate
/// directly with the template engines.
///
/// Using it in a request handler always returns successfully except when the [`Template::fairing()`]
/// wasn't attached to rocket.
///
/// # Usage
///
/// Ensure that the template [fairing](/rocket/fairing/) is attached to
/// your Rocket application:
///
/// ```rust
/// extern crate rocket;
/// extern crate rocket_contrib;
///
/// use rocket_contrib::Template;
///
/// fn main() {
///     rocket::ignite()
///         // ...
///         .attach(Template::fairing())
///         // ...
///     # ;
/// }
/// ```
///
/// The `TemplateEngine` type implements Rocket's `FromRequest` trait, so it can be
/// used as Parameter in a request handler:
///
/// ```rust,ignore
/// #[get("/")]
/// fn homepage(engine: TemplateEngine) -> Template {
///     if engine.template_exists("specific") {
///         return Template::render("specfic", json!({}));
///     }
///     Template::render("fallback", json!({}))
/// }
/// ```
pub struct TemplateMetadata<'a>(&'a Context);

impl<'a> TemplateMetadata<'a> {
    /// Returns `true` if the template with name `name` was loaded at start-up time. Otherwise,
    /// returns `false`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[get("/")]
    /// fn homepage(engine: TemplateEngine) -> Template {
    ///     if engine.template_exists("specific") {
    ///         return Template::render("specfic", json!({}));
    ///     }
    ///     Template::render("fallback", json!({}))
    /// }
    /// ```
    pub fn contains_template(&self, name: &str) -> bool {
        self.0.templates.contains_key(name)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for TemplateMetadata<'a> {
    type Error = ();

    fn from_request(request: &'a Request) -> Outcome<Self, ()> {
        request
            .guard::<State<Context>>()
            .succeeded()
            .and_then(|ctxt| Some(outcome::Outcome::Success(TemplateMetadata(ctxt.inner()))))
            .unwrap_or_else(|| {
                error_!("Uninitialized template context: missing fairing.");
                info_!("To use templates, you must attach `Template::fairing()`.");
                info_!("See the `Template` documentation for more information.");
                outcome::Outcome::Failure((Status::InternalServerError, ()))
            })
    }
}
