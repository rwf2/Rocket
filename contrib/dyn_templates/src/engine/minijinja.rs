use std::sync::Arc;
use std::path::Path;
use std::collections::HashMap;

use rocket::serde::Serialize;
use minijinja::{Environment, Error, ErrorKind, AutoEscape};

use crate::engine::Engine;

impl Engine for Environment<'static> {
    const EXT: &'static str = "j2";

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Self {
        let _templates = Arc::new(templates
            .map(|(k, p)| (k.to_owned(), p.to_owned()))
            .collect::<HashMap<_, _>>());

        let templates = _templates.clone();
        let mut env = Environment::new();
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

        let templates = _templates.clone();
        env.set_auto_escape_callback(move |name| {
            templates.get(name)
                .and_then(|path| path.to_str())
                .map(minijinja::default_auto_escape_callback)
                .unwrap_or(AutoEscape::None)
        });

        env
    }

    fn load<'a>(&mut self, _: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()> {
        Some(())
    }

    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String> {
        let template = match self.get_template(name) {
            Ok(template) => template,
            Err(e) => {
                error_!("Minijinja template '{name}' error: {e}");
                return None;
            }
        };

        match template.render(context) {
            Ok(result) => Some(result),
            Err(e) => {
                error_!("Error rendering Minijinja template '{name}': {e}");
                let mut error = &e as &dyn std::error::Error;
                while let Some(source) = error.source() {
                    error_!("caused by: {source}");
                    error = source;
                }
                None
            }
        }
    }
}
