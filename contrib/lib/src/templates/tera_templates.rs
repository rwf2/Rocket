use serde::Serialize;
use std::error::Error;

use crate::templates::{Engine, TemplateInfo};

pub use crate::templates::tera::{Context, Tera};

impl Engine for Tera {
    const EXT: &'static str = "tera";

    fn init(templates: &[(&str, &TemplateInfo)]) -> Option<Tera> {
        // Create the Tera instance.
        let mut tera = Tera::default();
        let ext = [".html.tera", ".htm.tera", ".xml.tera", ".html", ".htm", ".xml"];
        tera.autoescape_on(ext.to_vec());

        // Collect into a tuple of (name, path) for Tera.
        let tera_templates = templates.iter()
            .map(|&(name, info)| (&info.path, Some(name)))
            .collect::<Vec<_>>();

        // Finally try to tell Tera about all of the templates.
        if let Err(e) = tera.add_template_files(tera_templates) {
            error!("Failed to initialize Tera templating.");

            let mut error = Some(&e as &dyn Error);
            while let Some(err) = error {
                info_!("{}", err);
                error = err.source();
            }

            None
        } else {
            Some(tera)
        }
    }

    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String> {
        if self.get_template(name).is_err() {
            error_!("Tera template '{}' does not exist.", name);
            return None;
        };

        let tera_ctx = match Context::from_serialize(context) {
            Ok(ctx) => ctx,
            Err(_) => {
                error_!(
                    "Error generating context when rendering Tera template '{}'.",
                    name
                );
                return None;
            }
        };

        match Tera::render(self, name, &tera_ctx) {
            Ok(string) => Some(string),
            Err(e) => {
                error_!("Error rendering Tera template '{}'.", name);

                let mut error = Some(&e as &dyn Error);
                while let Some(err) = error {
                    error_!("{}", err);
                    error = err.source();
                }

                None
            }
        }
    }
}
