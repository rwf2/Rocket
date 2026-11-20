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
/// use rocket_dyn_templates::tera::{Value, Kwargs, State, TeraResult};
///
/// fn my_filter(value: &Value, _: Kwargs, _: &State) -> TeraResult<Value> {
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
    ///
    /// # Escaping
    ///
    /// HTML escaping is enabled for template names ending in `.html`, `.htm`,
    /// `.xml`, `.xhtml`, or `.svg`, regardless of ASCII case, optionally
    /// followed by `.tera`. Included templates use the escaping policy of the
    /// template that includes them. To change these defaults, call
    /// [`Tera::autoescape_on()`] or [`Tera::set_escape_fn()`] from a
    /// [`Templating::init()`] callback.
    ///
    /// Only the template's _name_ is considered. A template added from a file
    /// under a different name, as with `add_template_file(path, Some(name))`,
    /// is escaped only if `name` ends in one of the extensions above,
    /// regardless of the file's extension.
    ///
    /// # Template Loading
    ///
    /// Tera checks a template as soon as it's added: every filter, test,
    /// function, component, and template it uses must already exist.
    /// Similarly, delimiters cannot be changed once a template has been added.
    /// As a result, the order of [`Templating`] callbacks matters for Tera.
    /// See [customization](crate::Templating#customization) for which to use.
    ///
    /// Templates added together can use each other: Rocket adds all templates
    /// in `template_dir` at once, as does [`Tera::add_raw_templates()`]. A
    /// template in `template_dir` cannot, however, use a template added in an
    /// `on_load` callback. If templates must use each other in both
    /// directions, place them all in `template_dir`.
    ///
    /// [`Templating`]: crate::Templating
    /// [`Templating::init()`]: crate::Templating::init()
    #[cfg(feature = "tera")]
    pub tera: Tera,

    /// The Handlebars templating engine.
    ///
    /// This field is only available when the `handlebars` feature is enabled.
    /// When calling methods on the `Handlebars` instance, ensure you use types
    /// imported from `rocket_dyn_templates::handlebars` to avoid version
    /// mismatches.
    ///
    /// Handlebars uses HTML escaping regardless of the template name. To
    /// change this default, call [`Handlebars::register_escape_fn()`] from a
    /// [`Templating::init()`](crate::Templating::init()) callback.
    ///
    /// Handlebars parses a template when it's added but looks up helpers and
    /// partials only when rendering. As such, Handlebars templates can use
    /// helpers and templates registered in either
    /// [`Templating`](crate::Templating) callback.
    #[cfg(feature = "handlebars")]
    pub handlebars: Handlebars<'static>,

    /// The minijinja templating engine.
    ///
    /// This field is only available when the `minijinja` feature is enabled.
    /// When calling methods on the [`Environment`] instance, ensure you use
    /// types imported from `rocket_dyn_templates::minijinja` to avoid version
    /// mismatches.
    ///
    /// # Escaping
    ///
    /// HTML escaping is enabled for template names ending in `.html`, `.htm`,
    /// `.xml`, `.xhtml`, or `.svg`, regardless of ASCII case. The lowercase
    /// extensions `.json`, `.json5`, `.js`, `.yaml`, and `.yml` enable JSON
    /// escaping. A final `.j2`, `.jinja`, or `.jinja2` extension is ignored.
    ///
    /// Included templates use their own escaping policy. As such, HTML
    /// fragments should use an HTML extension: including a text template in an
    /// HTML template can render values without escaping them. To change these
    /// defaults, call [`Environment::set_auto_escape_callback()`] from a
    /// [`Templating::init()`](crate::Templating::init()) callback.
    ///
    /// # Template Loading
    ///
    /// MiniJinja parses templates in `template_dir` when they are first
    /// rendered, not when Rocket loads them, and looks up filters, tests,
    /// functions, and other templates only when rendering. As such, MiniJinja
    /// templates can use anything registered in either
    /// [`Templating`](crate::Templating) callback. A template added via
    /// [`Environment::add_template()`], however, is parsed immediately using
    /// the current syntax and escaping settings, so configure these first.
    ///
    /// Because templates in `template_dir` are parsed when rendered, a syntax
    /// error is only reported then. Rocket cannot detect such an error when it
    /// reloads templates, so the previous templates are not restored.
    ///
    /// Rocket loads templates in `template_dir` via the environment's loader
    /// and reloads them in debug mode, so MiniJinja's own automatic reloading
    /// is disabled. Replacing the loader via [`Environment::set_loader()`]
    /// makes templates in `template_dir` unavailable.
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
