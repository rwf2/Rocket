use std::collections::HashMap;
use std::path::Path;

use rocket::serde::Serialize;

use crate::template::TemplateInfo;

#[cfg(feature = "tera")]
mod tera;
#[cfg(feature = "tera")]
use ::tera::Tera;

#[cfg(feature = "handlebars")]
mod handlebars;
#[cfg(feature = "handlebars")]
use ::handlebars::Handlebars;

#[cfg(feature = "minijinja")]
mod minijinja;
#[cfg(feature = "minijinja")]
use ::minijinja::Environment;

pub(crate) trait Engine: Send + Sync + Sized + 'static {
    const EXT: &'static str;

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<Self>;
    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String>;
}

/// A structure exposing access to templating engines.
///
/// Calling methods on the exposed template engine types may require importing
/// types from the respective templating engine library. These types should be
/// imported from the reexported crate at the root of `rocket_dyn_templates` to
/// avoid version mismatches. For instance, when registering a Tera filter, the
/// [`tera::Value`] and [`tera::Result`] types are required. Import them from
/// `rocket_dyn_templates::tera`. The example below illustrates this:
///
/// ```rust
/// # #[cfg(feature = "tera")] {
/// # extern crate rocket_dyn_templates_community as rocket_dyn_templates;
/// use std::collections::HashMap;
///
/// use rocket_dyn_templates::{Template, Engines};
/// use rocket_dyn_templates::tera::{self, Value};
///
/// fn my_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
///     # /*
///     ...
///     # */ unimplemented!();
/// }
///
/// fn main() {
///     rocket::build()
///         // ...
///         .attach(Template::custom(|engines: &mut Engines| {
///             engines.tera.register_filter("my_filter", my_filter);
///         }))
///         // ...
///         # ;
/// }
/// # }
/// ```
///
/// [`tera::Value`]: crate::tera::Value
/// [`tera::Result`]: crate::tera::Result
pub struct Engines {
    /// A `Tera` templating engine.
    ///
    /// This field is only available when the `tera` feature is enabled. When
    /// calling methods on the `Tera` instance, ensure you use types imported
    /// from `rocket_dyn_templates::tera` to avoid version mismatches.
    #[cfg(feature = "tera")]
    pub tera: Tera,

    /// The Handlebars templating engine.
    ///
    /// This field is only available when the `handlebars` feature is enabled.
    /// When calling methods on the `Handlebars` instance, ensure you use types
    /// imported from `rocket_dyn_templates::handlebars` to avoid version
    /// mismatches.
    #[cfg(feature = "handlebars")]
    pub handlebars: Handlebars<'static>,

    /// The minijinja templating engine.
    ///
    /// This field is only available when the `minijinja` feature is enabled.
    /// When calling methods on the [`Environment`] instance, ensure you use
    /// types imported from `rocket_dyn_templates::minijinja` to avoid version
    /// mismatches.
    #[cfg(feature = "minijinja")]
    pub minijinja: Environment<'static>,
}

impl Engines {
    pub(crate) const ENABLED_EXTENSIONS: &'static [&'static str] = &[
        #[cfg(feature = "tera")]
        Tera::EXT,
        #[cfg(feature = "handlebars")]
        Handlebars::EXT,
        #[cfg(feature = "minijinja")]
        Environment::EXT,
    ];

    pub(crate) fn init(_templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
        fn inner<E: Engine>(templates: &HashMap<String, TemplateInfo>) -> Option<E> {
            let named_templates = templates
                .iter()
                .filter(|&(_, i)| i.engine_ext == E::EXT)
                .filter_map(|(k, i)| Some((k.as_str(), i.path.as_ref()?)))
                .map(|(k, p)| (k, p.as_path()));

            E::init(named_templates)
        }

        Some(Engines {
            #[cfg(feature = "tera")]
            tera: inner::<Tera>(_templates)?,
            #[cfg(feature = "handlebars")]
            handlebars: inner::<Handlebars<'static>>(_templates)?,
            #[cfg(feature = "minijinja")]
            minijinja: inner::<Environment<'static>>(_templates)?,
        })
    }

    pub(crate) fn render<C: Serialize>(
        &self,
        name: &str,
        info: &TemplateInfo,
        context: C,
    ) -> Option<String> {
        #[cfg(feature = "tera")]
        {
            if info.engine_ext == Tera::EXT {
                return Engine::render(&self.tera, name, context);
            }
        }

        #[cfg(feature = "handlebars")]
        {
            if info.engine_ext == Handlebars::EXT {
                return Engine::render(&self.handlebars, name, context);
            }
        }

        #[cfg(feature = "minijinja")]
        {
            if info.engine_ext == Environment::EXT {
                return Engine::render(&self.minijinja, name, context);
            }
        }

        None
    }

    /// Returns iterator over template (name, engine_extension).
    pub(crate) fn templates(&self) -> impl Iterator<Item = (&str, &'static str)> {
        #[cfg(feature = "tera")]
        let tera = self.tera.get_template_names().map(|name| (name, Tera::EXT));

        #[cfg(feature = "handlebars")]
        let handlebars = self
            .handlebars
            .get_templates()
            .keys()
            .map(|name| (name.as_str(), Handlebars::EXT));

        #[cfg(feature = "minijinja")]
        let minijinja = self
            .minijinja
            .templates()
            .map(|(name, _)| (name, Environment::EXT));

        #[cfg(not(feature = "tera"))]
        let tera = std::iter::empty();
        #[cfg(not(feature = "handlebars"))]
        let handlebars = std::iter::empty();
        #[cfg(not(feature = "minijinja"))]
        let minijinja = std::iter::empty();

        tera.chain(handlebars).chain(minijinja)
    }
}
