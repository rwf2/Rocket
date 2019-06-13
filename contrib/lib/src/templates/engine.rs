use std::collections::HashMap;

use serde::Serialize;

use crate::templates::TemplateInfo;

#[cfg(feature = "tera_templates")] use crate::templates::tera::Tera;
#[cfg(feature = "handlebars_templates")] use crate::templates::handlebars::Handlebars;

crate trait Engine: Send + Sync + 'static {
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
/// `rocket_contrib::templates::tera`. The example below illustrates this:
///
/// ```rust
/// # #[cfg(feature = "tera_templates")] {
/// use std::collections::HashMap;
///
/// use rocket_contrib::templates::{Template, Engines};
/// use rocket_contrib::templates::tera::{self, Value};
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
/// # }
/// ```
///
/// [`tera::Value`]: crate::templates::tera::Value
/// [`tera::Result`]: crate::templates::tera::Result
pub struct Engines {
    /// A `Tera` templating engine. This field is only available when the
    /// `tera_templates` feature is enabled. When calling methods on the `Tera`
    /// instance, ensure you use types imported from
    /// `rocket_contrib::templates::tera` to avoid version mismatches.
    #[cfg(feature = "tera_templates")]
    pub tera: Tera,
    /// The Handlebars templating engine. This field is only available when the
    /// `handlebars_templates` feature is enabled. When calling methods on the
    /// `Tera` instance, ensure you use types imported from
    /// `rocket_contrib::templates::handlebars` to avoid version mismatches.
    #[cfg(feature = "handlebars_templates")]
    pub handlebars: Handlebars,
}

impl Engines {
    crate const ENABLED_EXTENSIONS: &'static [&'static str] = &[
        #[cfg(feature = "tera_templates")] Tera::EXT,
        #[cfg(feature = "handlebars_templates")] Handlebars::EXT,
    ];

    crate fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
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

    crate fn render<C: Serialize>(
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
