use std::path::Path;
use std::collections::HashMap;

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

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Self;
    fn load<'a>(&mut self, templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()>;
    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String>;
}

/// A structure exposing access to templating engines.
///
/// Calling methods on the exposed template engine types may require importing
/// types from the respective templating engine library. These types should be
/// imported from the reexported crate at the root of `rocket_dyn_templates` to
/// avoid version mismatches. For instance, when registering a Tera filter, the
/// [`tera::Value`] and [`tera::TeraResult`] types are required. Import them
/// from `rocket_dyn_templates::tera`. The example below illustrates this:
///
/// ```rust
/// # #[cfg(feature = "tera")] {
/// use rocket_dyn_templates::{Templating, Engines};
/// use rocket_dyn_templates::tera::{self, Value};
///
/// fn my_filter(value: &Value, _: tera::Kwargs, _: &tera::State) -> tera::TeraResult<Value> {
///     # /*
///     ...
///     # */ unimplemented!();
/// }
///
/// fn main() {
///     rocket::build()
///         // ...
///         .attach(Templating::init(|engines: &mut Engines| {
///             engines.tera.register_filter("my_filter", my_filter);
///         }))
///         // ...
///         # ;
/// }
/// # }
/// ```
///
/// [`tera::Value`]: crate::tera::Value
/// [`tera::TeraResult`]: crate::tera::TeraResult
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
        #[cfg(feature = "tera")] Tera::EXT,
        #[cfg(feature = "handlebars")] Handlebars::EXT,
        #[cfg(feature = "minijinja")] Environment::EXT,
    ];

    fn files<E: Engine>(
        templates: &HashMap<String, TemplateInfo>
    ) -> impl Iterator<Item = (&str, &Path)> {
        templates.iter()
            .filter(|&(_, info)| info.engine_ext == E::EXT)
            .filter_map(|(name, info)| Some((name.as_str(), info.path.as_deref()?)))
    }

    pub(crate) fn init(templates: &HashMap<String, TemplateInfo>) -> Engines {
        Engines {
            #[cfg(feature = "tera")]
            tera: Tera::init(Self::files::<Tera>(templates)),
            #[cfg(feature = "handlebars")]
            handlebars: Handlebars::init(Self::files::<Handlebars<'static>>(templates)),
            #[cfg(feature = "minijinja")]
            minijinja: Environment::init(Self::files::<Environment<'static>>(templates)),
        }
    }

    pub(crate) fn load(&mut self, templates: &HashMap<String, TemplateInfo>) -> Option<()> {
        #[cfg(feature = "tera")]
        self.tera.load(Self::files::<Tera>(templates))?;
        #[cfg(feature = "handlebars")]
        self.handlebars.load(Self::files::<Handlebars<'static>>(templates))?;
        #[cfg(feature = "minijinja")]
        self.minijinja.load(Self::files::<Environment<'static>>(templates))?;
        Some(())
    }

    pub(crate) fn render<C: Serialize>(
        &self,
        name: &str,
        info: &TemplateInfo,
        context: C
    ) -> Option<String> {
        #[cfg(feature = "tera")] {
            if info.engine_ext == Tera::EXT {
                return Engine::render(&self.tera, name, context);
            }
        }

        #[cfg(feature = "handlebars")] {
            if info.engine_ext == Handlebars::EXT {
                return Engine::render(&self.handlebars, name, context);
            }
        }

        #[cfg(feature = "minijinja")] {
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
        let handlebars = self.handlebars.get_templates().keys()
                .map(|name| (name.as_str(), Handlebars::EXT));

        #[cfg(feature = "minijinja")]
        let minijinja = self.minijinja.templates()
            .map(|(name, _)| (name, Environment::EXT));

        #[cfg(not(feature = "tera"))] let tera = std::iter::empty();
        #[cfg(not(feature = "handlebars"))] let handlebars = std::iter::empty();
        #[cfg(not(feature = "minijinja"))] let minijinja = std::iter::empty();

        tera.chain(handlebars).chain(minijinja)
    }
}
