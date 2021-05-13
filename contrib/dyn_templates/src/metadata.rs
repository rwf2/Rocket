use rocket::{Request, Rocket, Ignite, Sentinel};
use rocket::http::Status;
use rocket::request::{self, FromRequest};

use crate::context::ContextManager;

/// Request guard for dynamically querying template metadata.
///
/// # Usage
///
/// The `Metadata` type implements Rocket's [`FromRequest`] trait, so it can be
/// used as a request guard in any request handler.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[macro_use] extern crate rocket_dyn_templates;
/// use rocket_dyn_templates::{Template, Metadata, context};
///
/// #[get("/")]
/// fn homepage(metadata: Metadata) -> Template {
///     // Conditionally render a template if it's available.
///     # let context = ();
///     if metadata.contains_template("some-template") {
///         Template::render("some-template", &context)
///     } else {
///         Template::render("fallback", &context)
///     }
/// }
///
/// fn main() {
///     rocket::build()
///         .attach(Template::fairing())
///         // ...
///     # ;
/// }
/// ```
pub struct Metadata<'a>(&'a ContextManager);

impl Metadata<'_> {
    /// Returns `true` if the template with the given `name` is currently
    /// loaded.  Otherwise, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// #
    /// use rocket_dyn_templates::Metadata;
    ///
    /// #[get("/")]
    /// fn handler(metadata: Metadata) {
    ///     // Returns `true` if the template with name `"name"` was loaded.
    ///     let loaded = metadata.contains_template("name");
    /// }
    /// ```
    pub fn contains_template(&self, name: &str) -> bool {
        self.0.context().templates.contains_key(name)
    }

    /// Returns `true` if template reloading is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// #
    /// use rocket_dyn_templates::Metadata;
    ///
    /// #[get("/")]
    /// fn handler(metadata: Metadata) {
    ///     // Returns `true` if template reloading is enabled.
    ///     let reloading = metadata.reloading();
    /// }
    /// ```
    pub fn reloading(&self) -> bool {
        self.0.is_reloading()
    }
}

impl Sentinel for Metadata<'_> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        if rocket.state::<ContextManager>().is_none() {
            let md = rocket::yansi::Paint::default("Metadata").bold();
            let fairing = rocket::yansi::Paint::default("Template::fairing()").bold();
            error!("requested `{}` guard without attaching `{}`.", md, fairing);
            info_!("To use or query templates, you must attach `{}`.", fairing);
            info_!("See the `Template` documentation for more information.");
            return true;
        }

        false
    }
}

/// Retrieves the template metadata. If a template fairing hasn't been attached,
/// an error is printed and an empty `Err` with status `InternalServerError`
/// (`500`) is returned.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Metadata<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        request.rocket().state::<ContextManager>()
            .map(|cm| request::Outcome::Success(Metadata(cm)))
            .unwrap_or_else(|| {
                error_!("Uninitialized template context: missing fairing.");
                info_!("To use templates, you must attach `Template::fairing()`.");
                info_!("See the `Template` documentation for more information.");
                request::Outcome::Failure((Status::InternalServerError, ()))
            })
    }
}
