use std::path::Path;

use handlebars::Handlebars;
use rocket::serde::Serialize;

use crate::engine::Engine;

impl Engine for Handlebars<'static> {
    const EXT: &'static str = "hbs";

    fn init<'a>(_: impl Iterator<Item = (&'a str, &'a Path)>) -> Self {
        Handlebars::new()
    }

    fn load<'a>(&mut self, templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<()> {
        let mut ok = true;
        for (name, path) in templates {
            if let Err(e) = self.register_template_file(name, path) {
                error!("Handlebars template '{}' failed to register.", name);
                error_!("{}", e);
                info_!("Template path: '{}'.", path.to_string_lossy());
                ok = false;
            }
        }

        ok.then_some(())
    }

    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String> {
        if self.get_template(name).is_none() {
            error_!("Handlebars template '{}' does not exist.", name);
            return None;
        }

        Handlebars::render(self, name, &context)
            .map_err(|e| error_!("Handlebars: {}", e))
            .ok()
    }
}
