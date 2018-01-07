use std::collections::HashMap;

use super::serde::Serialize;
use super::TemplateInfo;

#[cfg(feature = "tera_templates")] use super::tera_templates::Tera;
#[cfg(feature = "handlebars_templates")] use super::handlebars_templates::Handlebars;

pub trait Engine: Send + Sync + 'static {
    const EXT: &'static str;

    fn init(templates: &[(&str, &TemplateInfo)]) -> Option<Self> where Self: Sized;
    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String>;
}

/// A structure exposing access to templating engines.
///
/// Calling methods on the exposed template engine types may require importing
/// types from the respective templating engine library. These types should be
/// imported from the reexported crate at the root of `rocket_contrib` to avoid
/// version mismatches. For instance, when registering a Tera filter, the
/// [`tera::Value`] and [`tera::Result`] types are required. Import them from
/// `rocket_contrib::tera`. The example below illustrates this:
///
/// ```rust
/// use std::collections::HashMap;
///
/// use rocket_contrib::{Template, Engines};
/// use rocket_contrib::tera::{self, Value};
///
/// fn my_filter(value: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
///     # /*
///     ...
///     # */ unimplemented!();
/// }
///
/// Template::custom(|engines: &mut Engines| {
///     engines.tera.register_filter("my_filter", my_filter);
/// });
/// ```
///
/// [`tera::Value`]: https://docs.rs/tera/0.10.10/tera/enum.Value.html
/// [`tera::Result`]: https://docs.rs/tera/0.10.10/tera/type.Result.html
pub struct Engines {
    #[cfg(feature = "tera_templates")]
    /// A [`Tera`] structure. This field is only available when the
    /// `tera_templates` feature is enabled. When calling methods on the `Tera`
    /// instance, ensure you use types imported from `rocket_contrib::tera` to
    /// avoid version mismatches.
    ///
    /// [`Tera`]: https://docs.rs/tera/0.10.10/tera/struct.Tera.html
    pub tera: Tera,
    /// A [`Handlebars`] structure. This field is only available when the
    /// `handlebars_templates` feature is enabled. When calling methods on the
    /// `Tera` instance, ensure you use types
    /// imported from `rocket_contrib::handlebars` to avoid version mismatches.
    ///
    /// [`Handlebars`]:
    ///     https://docs.rs/handlebars/0.29.1/handlebars/struct.Handlebars.html
    #[cfg(feature = "handlebars_templates")]
    pub handlebars: Handlebars,
}

impl Engines {
    pub(crate) const ENABLED_EXTENSIONS: &'static [&'static str] = &[
        #[cfg(feature = "tera_templates")] Tera::EXT,
        #[cfg(feature = "handlebars_templates")] Handlebars::EXT,
    ];

    pub(crate) fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
        fn inner<E: Engine>(templates: &HashMap<String, TemplateInfo>) -> Option<E> {
            let named_templates = templates.iter()
                .filter(|&(_, i)| i.extension == E::EXT)
                .map(|(k, i)| (k.as_str(), i))
                .collect::<Vec<_>>();

            E::init(&*named_templates)
        }

        Some(Engines {
            #[cfg(feature = "tera_templates")]
            tera: match inner::<Tera>(templates) {
                Some(tera) => tera,
                None => return None
            },
            #[cfg(feature = "handlebars_templates")]
            handlebars: match inner::<Handlebars>(templates) {
                Some(hb) => hb,
                None => return None
            },
        })
    }

    pub(crate) fn render<C: Serialize>(
        &self,
        name: &str,
        info: &TemplateInfo,
        context: C
    ) -> Option<String> {
        #[cfg(feature = "tera_templates")]
        {
            if info.extension == Tera::EXT {
                return Engine::render(&self.tera, name, context);
            }
        }

        #[cfg(feature = "handlebars_templates")]
        {
            if info.extension == Handlebars::EXT {
                return Engine::render(&self.handlebars, name, context);
            }
        }

        None
    }
}
