use std::path::Path;
use std::error::Error;

use tera::{Context, Tera};
use rocket::serde::Serialize;

use crate::engine::Engine;

impl Engine for Tera {
    const EXT: &'static str = "tera";

    fn init<'a>(_: impl Iterator<Item = (&'a str, &'a Path)>) -> Self {
        // Create the Tera instance.
        let mut tera = Tera::default();
        let ext = [".html.tera", ".htm.tera", ".xml.tera", ".html", ".htm", ".xml"];
        tera.autoescape_on(ext.to_vec());
        tera
    }

    fn load<'a>(&mut self, templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()> {
        // Collect into a tuple of (name, path) for Tera. If we register one at
        // a time, it will complain about unregistered base templates.
        let files = templates.map(|(name, path)| (path, Some(name)));

        // Finally try to tell Tera about all of the templates.
        if let Err(e) = self.add_template_files(files) {
            error!("Failed to initialize Tera templating.");

            let mut error = Some(&e as &dyn Error);
            while let Some(err) = error {
                info_!("{}", err);
                error = err.source();
            }

            None
        } else {
            Some(())
        }
    }

    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String> {
        if self.get_template(name).is_err() {
            error_!("Tera template '{}' does not exist.", name);
            return None;
        };

        let tera_ctx = Context::from_serialize(context)
            .map_err(|e| error_!("Tera context error: {}.", e))
            .ok()?;

        match Tera::render(self, name, &tera_ctx) {
            Ok(string) => Some(string),
            Err(e) => {
                error_!("Error rendering Tera template '{name}': {e}");
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
