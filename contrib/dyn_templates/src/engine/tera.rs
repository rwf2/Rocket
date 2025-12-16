use std::error::Error;
use std::path::Path;

use rocket::serde::Serialize;
use tera::{Context, Tera};

use crate::engine::Engine;

impl Engine for Tera {
    const EXT: &'static str = "tera";

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<Self> {
        // Create the Tera instance.
        let mut tera = Tera::default();
        let ext = [
            ".html.tera",
            ".htm.tera",
            ".xml.tera",
            ".html",
            ".htm",
            ".xml",
        ];
        tera.autoescape_on(ext.to_vec());

        // Collect into a tuple of (name, path) for Tera. If we register one at
        // a time, it will complain about unregistered base templates.
        let files = templates.map(|(name, path)| (path, Some(name)));

        // Finally try to tell Tera about all of the templates.
        if let Err(e) = tera.add_template_files(files) {
            span_error!("templating", "Tera templating initialization failed" => {
                let mut error = Some(&e as &dyn Error);
                while let Some(err) = error {
                    error!("{err}");
                    error = err.source();
                }
            });

            None
        } else {
            Some(tera)
        }
    }

    fn render<C: Serialize>(&self, template: &str, context: C) -> Option<String> {
        if self.get_template(template).is_err() {
            error!(template, "requested template does not exist");
            return None;
        };

        let tera_ctx = Context::from_serialize(context)
            .map_err(|e| error!("Tera context error: {}.", e))
            .ok()?;

        match Tera::render(self, template, &tera_ctx) {
            Ok(string) => Some(string),
            Err(e) => {
                span_error!("templating", template, "failed to render Tera template" => {
                    let mut error = Some(&e as &dyn Error);
                    while let Some(err) = error {
                        error!("{err}");
                        error = err.source();
                    }
                });

                None
            }
        }
    }
}
