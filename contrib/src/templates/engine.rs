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

pub struct Engines {
    #[cfg(feature = "tera_templates")]
    tera: Tera,
    #[cfg(feature = "handlebars_templates")]
    handlebars: Handlebars,
}

impl Engines {
    pub const ENABLED_EXTENSIONS: &'static [&'static str] = &[
        #[cfg(feature = "tera_templates")] Tera::EXT,
        #[cfg(feature = "handlebars_templates")] Handlebars::EXT,
    ];

    pub fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
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

    pub fn render<C>(&self, name: &str, info: &TemplateInfo, c: C) -> Option<String>
        where C: Serialize
    {
        #[cfg(feature = "tera_templates")]
        {
            if info.extension == Tera::EXT {
                return Engine::render(&self.tera, name, c);
            }
        }

        #[cfg(feature = "handlebars_templates")]
        {
            if info.extension == Handlebars::EXT {
                return Engine::render(&self.handlebars, name, c);
            }
        }

        None
    }
}
