use std::path::Path;

use rocket::serde::Serialize;

use crate::engine::Engine;
pub use crate::handlebars::Handlebars;

impl Engine for Handlebars<'static> {
    const EXT: &'static str = "hbs";

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<Self> {
        let mut hb = Handlebars::new();
        let mut ok = true;
        for (name, path) in templates {
            if let Err(e) = hb.register_template_file(name, path) {
                error!("Handlebars template '{}' failed to register.", name);
                error_!("{}", e);
                info_!("Template path: '{}'.", path.to_string_lossy());
                ok = false;
            }
        }

        ok.then(|| hb)
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
