use rocket::{Request, State, Outcome};
use rocket::http::Status;
use rocket::request::{self, FromRequest};

use crate::templates::ContextManager;

/// Request guard for dynamiclly querying template metadata.
///
/// # Usage
///
/// The `Metadata` type implements Rocket's [`FromRequest`] trait, so it can be
/// used as a request guard in any request handler.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # #[macro_use] extern crate rocket_contrib;
/// use rocket_contrib::templates::{Template, Metadata};
///
/// #[get("/")]
/// fn homepage(metadata: Metadata) -> Template {
///     # use std::collections::HashMap;
///     # let context: HashMap<String, String> = HashMap::new();
///     // Conditionally render a template if it's available.
///     if metadata.contains_template("some-template") {
///         Template::render("some-template", &context)
///     } else {
///         Template::render("fallback", &context)
///     }
/// }
///
///
/// fn main() {
///     rocket::ignite()
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
    /// # #![feature(proc_macro_hygiene, decl_macro)]
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_contrib;
    /// #
    /// use rocket_contrib::templates::Metadata;
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
    /// # #![feature(proc_macro_hygiene, decl_macro)]
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_contrib;
    /// #
    /// use rocket_contrib::templates::Metadata;
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

/// Retrieves the template metadata. If a template fairing hasn't been attached,
/// an error is printed and an empty `Err` with status `InternalServerError`
/// (`500`) is returned.
impl<'a> FromRequest<'a, '_> for Metadata<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'_>) -> request::Outcome<Self, ()> {
        request.guard::<State<'_, ContextManager>>()
            .succeeded()
            .and_then(|cm| Some(Outcome::Success(Metadata(cm.inner()))))
            .unwrap_or_else(|| {
                error_!("Uninitialized template context: missing fairing.");
                info_!("To use templates, you must attach `Template::fairing()`.");
                info_!("See the `Template` documentation for more information.");
                Outcome::Failure((Status::InternalServerError, ()))
            })
    }
}
