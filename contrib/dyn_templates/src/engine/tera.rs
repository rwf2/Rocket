use std::path::Path;
use std::error::Error;

use tera::{Context, Tera};
use rocket::serde::Serialize;

use crate::engine::Engine;

impl Engine for Tera {
    const EXT: &'static str = "tera";

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<Self> {
        // Create the Tera instance.
        let mut tera = Tera::default();

        // Collect into a tuple of (name, path) for Tera. If we register one at
        // a time, it will complain about unregistered base templates.
        let files: Vec<_> = templates.map(|(name, path)| (path, Some(name))).collect();

        // Tera autoescapes a template if its _name_ ends with a registered
        // suffix, but the names of templates from files have no extension. To
        // escape templates with an HTML or XML path, we register their names.
        let escape = |path: &Path| path.to_str().is_some_and(|path| {
            [".html.tera", ".htm.tera", ".xml.tera"].iter().any(|ext| path.ends_with(ext))
        });

        let mut suffixes = vec![".html", ".htm", ".xml"];
        suffixes.extend(files.iter().filter(|(p, _)| escape(p)).filter_map(|(_, n)| *n));

        // A suffix matches any name ending with it, so a template whose name
        // ends with an escaped template's name is also escaped. This is safe
        // but may be unexpected, so we warn about it.
        for name in files.iter().filter(|(p, _)| !escape(p)).filter_map(|(_, n)| *n) {
            if let Some(suffix) = suffixes.iter().find(|s| name.ends_with(**s)) {
                warn!(template = name, suffix, "Tera template will be HTML escaped\n\
                    its name ends with the name or extension of an HTML/XML template\n\
                    rename the template to avoid escaping");
            }
        }

        tera.autoescape_on(suffixes.into_iter().map(String::from));

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
        if !self.contains_template(template) {
            error!(template, "requested template does not exist");
            return None;
        };

        let tera_ctx = Context::from_serialize(&context)
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
