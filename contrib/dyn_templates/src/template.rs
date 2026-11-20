use std::borrow::Cow;
use std::path::PathBuf;

use rocket::{Rocket, Orbit, Ignite, Sentinel};
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::http::{ContentType, Status};
use rocket::figment::{value::Value, error::Error};
use rocket::trace::Trace;
use rocket::serde::Serialize;

use crate::context::{Context, ContextManager};

pub(crate) const DEFAULT_TEMPLATE_DIR: &str = "templates";

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
    value: Result<Value, Error>,
}

#[derive(Debug)]
pub(crate) struct TemplateInfo {
    /// The complete path, including `template_dir`, to this template, if any.
    pub(crate) path: Option<PathBuf>,
    /// The extension for the engine of this template.
    pub(crate) engine_ext: &'static str,
    /// The extension before the engine extension in the template, if any.
    pub(crate) data_type: ContentType
}

impl Template {
    /// Render the template named `name` with the context `context`. The
    /// `context` is typically created using the [`context!()`](crate::context!)
    /// macro, but it can be of any type that implements `Serialize`, such as
    /// `HashMap` or a custom `struct`.
    ///
    /// To render a template directly into a string, use
    /// [`Metadata::render()`](crate::Metadata::render()).
    ///
    /// # Examples
    ///
    /// Using the `context` macro:
    ///
    /// ```rust
    /// use rocket_dyn_templates::{Template, context};
    ///
    /// let template = Template::render("index.html", context! {
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
    /// let template = Template::render("index.html", context);
    /// ```
    #[inline]
    pub fn render<S, C>(name: S, context: C) -> Template
        where S: Into<Cow<'static, str>>, C: Serialize
    {
        Template {
            name: name.into(),
            value: Value::serialize(context),
        }
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
    /// `None` is also returned if a `Templating` fairing has not been attached.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// use std::collections::HashMap;
    ///
    /// use rocket_dyn_templates::{Template, Templating};
    /// use rocket::local::blocking::Client;
    ///
    /// fn main() {
    ///     let rocket = rocket::build().attach(Templating::new());
    ///     let client = Client::untracked(rocket).expect("valid rocket");
    ///
    ///     // Create a `context`. Here, just an empty `HashMap`.
    ///     let mut context = HashMap::new();
    ///     # context.insert("test", "test");
    ///     let template =
    ///         Template::show(client.rocket(), "index.html", context);
    /// }
    /// ```
    #[inline]
    pub fn show<S, C>(rocket: &Rocket<Orbit>, name: S, context: C) -> Option<String>
        where S: Into<Cow<'static, str>>, C: Serialize
    {
        let ctxt = rocket.state::<ContextManager>()
            .map(ContextManager::context)
            .or_else(|| {
                error!("Uninitialized template context: missing fairing.\n\
                    To use templates, you must attach `Templating::new()`.\n\
                    See the `Templating` documentation for more information.");

                None
            })?;

        Template::render(name, context).finalize(&ctxt).ok().map(|v| v.1)
    }

    /// Actually render this template given a template context. This method is
    /// called by the `Template` `Responder` implementation as well as
    /// `Template::show()`.
    #[inline(always)]
    pub(crate) fn finalize(self, ctxt: &Context) -> Result<(ContentType, String), Status> {
        let template = &*self.name;
        let info = ctxt.templates.get(template).ok_or_else(|| {
            let ts: Vec<_> = ctxt.templates.keys().map(|s| s.as_str()).collect();
            error!(
                %template, search_path = %ctxt.root.display(), known_templates = ?ts,
                "requested template not found"
            );

            for known in ts.iter().filter(|t| is_name_without_extension(template, t)) {
                info!("Did you mean '{known}'? Names include the file type extension.");
            }

            Status::InternalServerError
        })?;

        let value = self.value.map_err(|e| {
            span_error!("templating", "template context failed to serialize" => e.trace_error());
            Status::InternalServerError
        })?;

        let string = ctxt.engines.render(template, info, value).ok_or_else(|| {
            error!(template, "template failed to render");
            Status::InternalServerError
        })?;

        Ok((info.data_type.clone(), string))
    }
}

/// Returns `true` if `known` is `name` followed by a file type extension, as
/// in `index` and `index.html`.
fn is_name_without_extension(name: &str, known: &str) -> bool {
    known.strip_prefix(name)
        .and_then(|rest| rest.strip_prefix('.'))
        .is_some_and(|ext| !ext.is_empty() && !ext.contains(['.', '/']))
}

/// Returns a response with the Content-Type derived from the template's
/// extension and a fixed-size body containing the rendered template. If
/// rendering fails, an `Err` of `Status::InternalServerError` is returned.
impl<'r> Responder<'r, 'static> for Template {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let ctxt = req.rocket()
            .state::<ContextManager>()
            .ok_or_else(|| {
                error!(
                    "Uninitialized template context: missing fairing.\n\
                    To use templates, you must attach `Templating::new()`.\n\
                    See the `Templating` documentation for more information."
                );

                Status::InternalServerError
            })?;

        self.finalize(&ctxt.context())?.respond_to(req)
    }
}

impl Sentinel for Template {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        if rocket.state::<ContextManager>().is_none() {
            error!(
                "returning `Template` responder without attaching `Templating::new()`.\n\
                To use or query templates, you must attach `Templating::new()`.\n\
                See the `Templating` documentation for more information."
            );

            return true;
        }

        false
    }
}

/// A macro to easily create a template rendering context.
///
/// Invocations of this macro expand to a value of an anonymous type which
/// implements [`Serialize`]. Fields can be literal expressions or variables
/// captured from a surrounding scope, as long as all fields implement
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
///     Template::render("index.html", context! {
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
///     Template::render("index.html", IndexContext {
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
        use ::std::result::Result;

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

#[cfg(test)]
mod tests {
    use super::is_name_without_extension;

    #[test]
    fn name_without_extension() {
        assert!(is_name_without_extension("index", "index.html"));
        assert!(is_name_without_extension("dir/index", "dir/index.html"));
        assert!(is_name_without_extension("index.template", "index.template.html"));

        assert!(!is_name_without_extension("index", "index"));
        assert!(!is_name_without_extension("index", "index."));
        assert!(!is_name_without_extension("index", "indexes.html"));
        assert!(!is_name_without_extension("index", "index.template.html"));
        assert!(!is_name_without_extension("index", "index.d/page"));
        assert!(!is_name_without_extension("index.html", "index"));
    }
}
