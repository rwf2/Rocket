use std::path::Path;
use std::error::Error;

use tera::{Context, Tera};
use rocket::serde::Serialize;

use crate::engine::Engine;

/// Extensions of template names for which HTML escaping is enabled.
const MARKUP_EXTENSIONS: &[&str] = &["html", "htm", "xml", "xhtml", "svg"];

/// Returns the suffixes of template names for which HTML escaping is enabled:
/// every ASCII case variant of each markup extension, on its own (`.HtMl`) and
/// followed by the engine extension (`.HtMl.tera`).
///
/// Tera only matches suffixes exactly, so each variant is listed. The engine
/// extension is included for templates registered directly with Tera under a
/// name that ends with it, such as `email.html.tera`.
fn autoescape_suffixes() -> Vec<String> {
    let mut suffixes = vec![];
    for ext in MARKUP_EXTENSIONS {
        for mask in 0..1u32 << ext.len() {
            let variant = ext.chars().enumerate()
                .map(|(i, c)| if mask & (1 << i) != 0 { c.to_ascii_uppercase() } else { c })
                .collect::<String>();

            suffixes.push(format!(".{variant}.{}", Tera::EXT));
            suffixes.push(format!(".{variant}"));
        }
    }

    suffixes
}

impl Engine for Tera {
    const EXT: &'static str = "tera";

    fn init<'a>(_: impl Iterator<Item = (&'a str, &'a Path)>) -> Self {
        // Create the Tera instance.
        let mut tera = Tera::default();
        tera.autoescape_on(autoescape_suffixes());
        tera
    }

    fn load<'a>(&mut self, templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()> {
        // Collect into a tuple of (name, path) for Tera. If we register one at
        // a time, it will complain about unregistered base templates.
        let files = templates.map(|(name, path)| (path, Some(name)));

        // Finally try to tell Tera about all of the templates.
        if let Err(e) = self.add_template_files(files) {
            span_error!("templating", "Tera templating initialization failed" => {
                let mut error = Some(&e as &dyn Error);
                while let Some(err) = error {
                    error!("{err}");
                    error = err.source();
                }
            });

            None
        } else {
            Some(())
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
