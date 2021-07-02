//! Dynamic templating engine support for Rocket.
//!
//! This crate adds support for dynamic template rendering to Rocket. It
//! automatically discovers templates, provides a `Responder` to render
//! templates, and automatically reloads templates when compiled in debug mode.
//! At present, it supports [Handlebars] and [Tera].
//!
//! # Usage
//!
//!   1. Enable the `rocket_dyn_templates` feature corresponding to your
//!      templating engine(s) of choice:
//!
//!      ```toml
//!      [dependencies.rocket_dyn_templates]
//!      version = "0.1.0-rc.1"
//!      features = ["handlebars", "tera"]
//!      ```
//!
//!   1. Write your template files in Handlebars (`.hbs`) and/or Tera (`.tera`)
//!      in the configurable `template_dir` directory (default:
//!      `{rocket_root}/templates`).
//!
//!   2. Attach `Template::fairing()` return a `Template` using
//!      `Template::render()`, supplying the name of the template file **minus
//!      the last two extensions**:
//!
//!      ```rust
//!      # #[macro_use] extern crate rocket;
//!      use rocket_dyn_templates::Template;
//!
//!      #[launch]
//!      fn rocket() -> _ {
//!          rocket::build().attach(Template::fairing())
//!      }
//!
//!      #[get("/")]
//!      fn index() -> Template {
//!          # let context = ();
//!          Template::render("template-name", &context)
//!      }
//!      ```
//!
//! ## Naming
//!
//! Templates discovered by Rocket are _renamed_ from their file name to their
//! file name **without the last two extensions**. As such, refer to a template
//! with file name `foo.html.hbs` or `foo.html.tera` as `foo`. See
//! [Discovery](#discovery) for more.
//!
//! Templates that are _not_ discovered by Rocket, such as those registered
//! directly via [`Template::custom()`], are _not_ renamed. Use the name with
//! which the template was originally registered.
//!
//! ## Content Type
//!
//! The `Content-Type` of the response is automatically determined by the
//! non-engine extension of the template name or `text/plain` if there is no
//! extension or the extension is unknown. For example, for a discovered
//! template with file name `foo.html.hbs` or a manually registered template
//! with name ending in `foo.html`, the `Content-Type` is automatically set to
//! `ContentType::HTML`.
//!
//! ## Discovery
//!
//! Template names passed in to [`Template::render()`] must correspond to a
//! previously discovered template in the configured template directory. The
//! template directory is configured via the `template_dir` configuration
//! parameter and defaults to `templates/`. The path set in `template_dir` is
//! relative to the Rocket configuration file. See the [configuration
//! chapter](https://rocket.rs/v0.5-rc/guide/configuration) of the guide for more
//! information on configuration.
//!
//! The corresponding templating engine used for a given template is based on a
//! template's extension. At present, this library supports the following
//! engines and extensions:
//!
//! | Engine       | Version | Extension |
//! |--------------|---------|-----------|
//! | [Tera]       | 1       | `.tera`   |
//! | [Handlebars] | 3       | `.hbs`    |
//!
//! [Tera]: https://docs.rs/crate/tera/1
//! [Handlebars]: https://docs.rs/crate/handlebars/3
//!
//! Any file that ends with one of these extension will be discovered and
//! rendered with the corresponding templating engine. The _name_ of the
//! template will be the path to the template file relative to `template_dir`
//! minus at most two extensions. The following table contains examples of this
//! mapping:
//!
//! | example template path                         | template name         |
//! |-----------------------------------------------|-----------------------|
//! | {template_dir}/index.html.hbs                 | index                 |
//! | {template_dir}/index.tera                     | index                 |
//! | {template_dir}/index.hbs                      | index                 |
//! | {template_dir}/dir/index.hbs                  | dir/index             |
//! | {template_dir}/dir/index.html.tera            | dir/index             |
//! | {template_dir}/index.template.html.hbs        | index.template        |
//! | {template_dir}/subdir/index.template.html.hbs | subdir/index.template |
//!
//! The recommended naming scheme is to use two extensions: one for the file
//! type, and one for the template extension. This means that template
//! extensions should look like: `.html.hbs`, `.html.tera`, `.xml.hbs`, etc.
//!
//! ## Template Fairing and Customization
//!
//! Template discovery is actualized by the template fairing, which itself is
//! created via [`Template::fairing()`], [`Template::custom()`], or
//! [`Template::try_custom()`], the latter two allowing customizations to
//! templating engines such as registering template helpers and register
//! templates from strings.
//!
//! In order for _any_ templates to be rendered, the template fairing _must_ be
//! [attached](rocket::Rocket::attach()) to the running Rocket instance. Failure
//! to do so will result in an ignite-time error.
//!
//! ## Rendering
//!
//! Templates are rendered with the `render` method. The method takes in the
//! name of a template and a context to render the template with. The context
//! can be any type that implements [`Serialize`] and would serialize to an
//! `Object` value. The [`context!`] macro can also be used to create inline
//! `Serialize`-able context objects.
//!
//! ## Automatic Reloading
//!
//! In debug mode (without the `--release` flag passed to `cargo`), templates
//! will be automatically reloaded from disk if any changes have been made to
//! the templates directory since the previous request. In release builds,
//! template reloading is disabled to improve performance and cannot be enabled.

#![doc(html_root_url = "https://api.rocket.rs/v0.5-rc/rocket_dyn_templates")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

#[macro_use] extern crate rocket;

#[cfg(not(any(feature = "tera", feature = "handlebars")))]
compile_error!("at least one of \"tera\" or \"handlebars\" features must be enabled");

/// The tera templating engine library, reexported.
#[doc(inline)]
#[cfg(feature = "tera")]
pub use tera_ as tera;

#[cfg(feature = "tera")]
mod tera_templates;

/// The handlebars templating engine library, reexported.
#[doc(inline)]
#[cfg(feature = "handlebars")]
pub use handlebars_ as handlebars;

#[cfg(feature = "handlebars")]
mod handlebars_templates;

mod engine;
mod fairing;
mod context;
mod metadata;

pub use self::engine::Engines;
pub use self::metadata::Metadata;

use self::fairing::TemplateFairing;
use self::context::{Context, ContextManager};

use std::borrow::Cow;
use std::path::PathBuf;

#[doc(hidden)]
pub use rocket::serde;

use rocket::{Rocket, Orbit, Ignite, Sentinel};
use rocket::request::Request;
use rocket::fairing::Fairing;
use rocket::response::{self, Responder};
use rocket::http::{ContentType, Status};
use rocket::figment::{value::Value, error::Error};
use rocket::serde::Serialize;

const DEFAULT_TEMPLATE_DIR: &str = "templates";

/// Responder that renders a dynamic template.
///
/// `Template` serves as a _proxy_ type for rendering a template and _does not_
/// contain the rendered template itself. The template is lazily rendered, at
/// response time. To render a template greedily, use [`Template::show()`].
///
/// See the [crate root](crate) for usage details.
#[derive(Debug)]
pub struct Template {
    name: Cow<'static, str>,
    value: Result<Value, Error>
}

#[derive(Debug)]
pub(crate) struct TemplateInfo {
    /// The complete path, including `template_dir`, to this template, if any.
    path: Option<PathBuf>,
    /// The extension for the engine of this template.
    engine_ext: &'static str,
    /// The extension before the engine extension in the template, if any.
    data_type: ContentType
}

impl Template {
    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// This fairing, or the one returned by [`Template::custom()`], _must_ be
    /// attached to any `Rocket` instance that wishes to render templates.
    /// Failure to attach this fairing will result in a "Uninitialized template
    /// context: missing fairing." error message when a template is attempted to
    /// be rendered.
    ///
    /// If you wish to customize the internal templating engines, use
    /// [`Template::custom()`] instead.
    ///
    /// # Example
    ///
    /// To attach this fairing, simple call `attach` on the application's
    /// `Rocket` instance with `Template::fairing()`:
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Template;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Template::fairing())
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn fairing() -> impl Fairing {
        Template::custom(|_| {})
    }

    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// Unlike [`Template::fairing()`], this method allows you to configure
    /// templating engines via the function `f`. Note that only the enabled
    /// templating engines will be accessible from the `Engines` type.
    ///
    /// This method does not allow the function `f` to fail. If `f` is fallible,
    /// use [`Template::try_custom()`] instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Template;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Template::custom(|engines| {
    ///             // engines.handlebars.register_helper ...
    ///         }))
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn custom<F: Send + Sync + 'static>(f: F) -> impl Fairing
        where F: Fn(&mut Engines)
    {
        Self::try_custom(move |engines| { f(engines); Ok(()) })
    }

    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// This variant of [`Template::custom()`] allows a fallible `f`. If `f`
    /// returns an error during initialization, it will cancel the launch. If
    /// `f` returns an error during template reloading (in debug mode), then the
    /// newly-reloaded templates are discarded.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Template;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Template::try_custom(|engines| {
    ///             // engines.handlebars.register_helper ...
    ///             Ok(())
    ///         }))
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn try_custom<F: Send + Sync + 'static>(f: F) -> impl Fairing
        where F: Fn(&mut Engines) -> Result<(), Box<dyn std::error::Error>>
    {
        TemplateFairing { callback: Box::new(f) }
    }

    /// Render the template named `name` with the context `context`. The
    /// `context` is typically created using the [`context!`] macro, but it can
    /// be of any type that implements `Serialize`, such as `HashMap` or a
    /// custom `struct`.
    ///
    /// # Examples
    ///
    /// Using the `context` macro:
    ///
    /// ```rust
    /// use rocket_dyn_templates::{Template, context};
    ///
    /// let template = Template::render("index", context! {
    ///     foo: "Hello, world!",
    /// });
    /// ```
    ///
    /// Using a `HashMap` as the context:
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use rocket_dyn_templates::Template;
    ///
    /// // Create a `context` from a `HashMap`.
    /// let mut context = HashMap::new();
    /// context.insert("foo", "Hello, world!");
    ///
    /// let template = Template::render("index", context);
    /// ```
    #[inline]
    pub fn render<S, C>(name: S, context: C) -> Template
        where S: Into<Cow<'static, str>>, C: Serialize
    {
        Template { name: name.into(), value: Value::serialize(context) }
    }

    /// Render the template named `name` with the context `context` into a
    /// `String`. This method should **not** be used in any running Rocket
    /// application. This method should only be used during testing to validate
    /// `Template` responses. For other uses, use [`render()`](#method.render)
    /// instead.
    ///
    /// The `context` can be of any type that implements `Serialize`. This is
    /// typically a `HashMap` or a custom `struct`.
    ///
    /// Returns `Some` if the template could be rendered. Otherwise, returns
    /// `None`. If rendering fails, error output is printed to the console.
    /// `None` is also returned if a `Template` fairing has not been attached.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// use std::collections::HashMap;
    ///
    /// use rocket_dyn_templates::Template;
    /// use rocket::local::blocking::Client;
    ///
    /// fn main() {
    ///     let rocket = rocket::build().attach(Template::fairing());
    ///     let client = Client::untracked(rocket).expect("valid rocket");
    ///
    ///     // Create a `context`. Here, just an empty `HashMap`.
    ///     let mut context = HashMap::new();
    ///     # context.insert("test", "test");
    ///     let template = Template::show(client.rocket(), "index", context);
    /// }
    /// ```
    #[inline]
    pub fn show<S, C>(rocket: &Rocket<Orbit>, name: S, context: C) -> Option<String>
        where S: Into<Cow<'static, str>>, C: Serialize
    {
        let ctxt = rocket.state::<ContextManager>().map(ContextManager::context).or_else(|| {
            warn!("Uninitialized template context: missing fairing.");
            info!("To use templates, you must attach `Template::fairing()`.");
            info!("See the `Template` documentation for more information.");
            None
        })?;

        Template::render(name, context).finalize(&ctxt).ok().map(|v| v.0)
    }

    /// Actually render this template given a template context. This method is
    /// called by the `Template` `Responder` implementation as well as
    /// `Template::show()`.
    #[inline(always)]
    fn finalize(self, ctxt: &Context) -> Result<(String, ContentType), Status> {
        let name = &*self.name;
        let info = ctxt.templates.get(name).ok_or_else(|| {
            let ts: Vec<_> = ctxt.templates.keys().map(|s| s.as_str()).collect();
            error_!("Template '{}' does not exist.", name);
            info_!("Known templates: {}.", ts.join(", "));
            info_!("Searched in {:?}.", ctxt.root);
            Status::InternalServerError
        })?;

        let value = self.value.map_err(|e| {
            error_!("Template context failed to serialize: {}.", e);
            Status::InternalServerError
        })?;

        let string = ctxt.engines.render(name, &info, value).ok_or_else(|| {
            error_!("Template '{}' failed to render.", name);
            Status::InternalServerError
        })?;

        Ok((string, info.data_type.clone()))
    }
}

/// Returns a response with the Content-Type derived from the template's
/// extension and a fixed-size body containing the rendered template. If
/// rendering fails, an `Err` of `Status::InternalServerError` is returned.
impl<'r> Responder<'r, 'static> for Template {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let (render, content_type) = {
            let ctxt = req.rocket().state::<ContextManager>().ok_or_else(|| {
                error_!("Uninitialized template context: missing fairing.");
                info_!("To use templates, you must attach `Template::fairing()`.");
                info_!("See the `Template` documentation for more information.");
                Status::InternalServerError
            })?.context();

            self.finalize(&ctxt)?
        };

        (content_type, render).respond_to(req)
    }
}

impl Sentinel for Template {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        if rocket.state::<ContextManager>().is_none() {
            let template = rocket::yansi::Paint::default("Template").bold();
            let fairing = rocket::yansi::Paint::default("Template::fairing()").bold();
            error!("returning `{}` responder without attaching `{}`.", template, fairing);
            info_!("To use or query templates, you must attach `{}`.", fairing);
            info_!("See the `Template` documentation for more information.");
            return true;
        }

        false
    }
}

/// A macro to easily create a template rendering context.
///
/// Invocations of this macro expand to a value of an anonymous type which
/// implements [`serde::Serialize`]. Fields can be literal expressions or
/// variables captured from a surrounding scope, as long as all fields implement
/// `Serialize`.
///
/// # Examples
///
/// The following code:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket_dyn_templates::{Template, context};
/// #[get("/<foo>")]
/// fn render_index(foo: u64) -> Template {
///     Template::render("index", context! {
///         // Note that shorthand field syntax is supported.
///         // This is equivalent to `foo: foo,`
///         foo,
///         bar: "Hello world",
///     })
/// }
/// ```
///
/// is equivalent to the following, but without the need to manually define an
/// `IndexContext` struct:
///
/// ```rust
/// # use rocket_dyn_templates::Template;
/// # use rocket::serde::Serialize;
/// # use rocket::get;
/// #[derive(Serialize)]
/// # #[serde(crate = "rocket::serde")]
/// struct IndexContext<'a> {
///     foo: u64,
///     bar: &'a str,
/// }
///
/// #[get("/<foo>")]
/// fn render_index(foo: u64) -> Template {
///     Template::render("index", IndexContext {
///         foo,
///         bar: "Hello world",
///     })
/// }
/// ```
///
/// ## Nesting
///
/// Nested objects can be created by nesting calls to `context!`:
///
/// ```rust
/// # use rocket_dyn_templates::context;
/// # fn main() {
/// let ctx = context! {
///     planet: "Earth",
///     info: context! {
///         mass: 5.97e24,
///         radius: "6371 km",
///         moons: 1,
///     },
/// };
/// # }
/// ```
#[macro_export]
macro_rules! context {
    ($($key:ident $(: $value:expr)?),*$(,)?) => {{
        use $crate::serde::ser::{Serialize, Serializer, SerializeMap};
        use ::std::fmt::{Debug, Formatter};

        #[allow(non_camel_case_types)]
        struct ContextMacroCtxObject<$($key: Serialize),*> {
            $($key: $key),*
        }

        #[allow(non_camel_case_types)]
        impl<$($key: Serialize),*> Serialize for ContextMacroCtxObject<$($key),*> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: Serializer,
            {
                let mut map = serializer.serialize_map(None)?;
                $(map.serialize_entry(stringify!($key), &self.$key)?;)*
                map.end()
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($key: Debug + Serialize),*> Debug for ContextMacroCtxObject<$($key),*> {
            fn fmt(&self, f: &mut Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct("context!")
                    $(.field(stringify!($key), &self.$key))*
                    .finish()
            }
        }

        ContextMacroCtxObject {
            $($key $(: $value)?),*
        }
    }};
}
