use std::path::Path;
use std::collections::HashMap;

use rocket::serde::Serialize;
use minijinja::{Environment, Error, ErrorKind, AutoEscape};
use minijinja::value::Serde;

use crate::engine::Engine;

impl Engine for Environment<'static> {
    const EXT: &'static str = "j2";

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Self {
        let templates = templates
            .map(|(name, path)| (name.to_owned(), path.to_owned()))
            .collect::<HashMap<_, _>>();

        let mut env = Environment::new();
        env.set_auto_reload(false);
        env.set_loader(move |name| {
            let Some(path) = templates.get(name) else {
                return Ok(None);
            };

            match std::fs::read_to_string(path) {
                Ok(result) => Ok(Some(result)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(
                    Error::new(ErrorKind::InvalidOperation, "template read failed").with_source(e)
                ),
            }
        });

        env.set_auto_escape_callback(|name| {
            let Some((stem, ext)) = name.rsplit_once('.') else {
                return AutoEscape::None;
            };

            let ext = match ext {
                "j2" | "jinja" | "jinja2" => match stem.rsplit_once('.') {
                    Some((_, ext)) => ext,
                    None => return AutoEscape::None,
                },
                _ => ext,
            };

            if ["html", "htm", "xml", "xhtml", "svg"].iter().any(|s| ext.eq_ignore_ascii_case(s)) {
                AutoEscape::Html
            } else {
                minijinja::default_auto_escape_callback(name)
            }
        });

        env
    }

    fn load<'a>(&mut self, _: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()> {
        Some(())
    }

    fn render<C: Serialize>(&self, template: &str, context: C) -> Option<String> {
        let templ = match self.get_template(template) {
            Ok(templ) => templ,
            Err(e) => {
                span_error!("templating", template, "failed to load Minijinja template" => {
                    let mut error = Some(&e as &dyn std::error::Error);
                    while let Some(err) = error {
                        error!("{err}");
                        error = err.source();
                    }
                });

                return None;
            }
        };

        match templ.render(Serde(context)) {
            Ok(result) => Some(result),
            Err(e) => {
                span_error!("templating", template, "failed to render Minijinja template" => {
                    let mut error = Some(&e as &dyn std::error::Error);
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
