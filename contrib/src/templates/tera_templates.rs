extern crate tera;

use std::path::PathBuf;

use super::serde::Serialize;
use super::{TemplateInfo, TEMPLATE_DIR};

lazy_static! {
    static ref TERA: Result<tera::Tera, String> = {
        let path: PathBuf = [&*TEMPLATE_DIR, "**", "*.tera"].iter().collect();
        let ext = [".html.tera", ".htm.tera", ".xml.tera", ".html", ".htm", ".xml"];
        tera::Tera::new(path.to_str().unwrap())
            .map(|mut tera| {
                tera.autoescape_on(ext.to_vec());
                tera
            })
            .map_err(|e| format!("{:?}", e))
    };
}

pub const EXT: &'static str = "tera";

pub fn render<T>(name: &str, info: &TemplateInfo, context: &T) -> Option<String>
    where T: Serialize
{
    let tera = match *TERA {
        Ok(ref tera) => tera,
        Err(ref e) => {
            error_!("Tera failed to initialize: {}.", e);
            return None;
        }
    };

    let template_name = &info.path.to_string_lossy();
    if tera.get_template(template_name).is_err() {
        error_!("Tera template '{}' does not exist.", template_name);
        return None;
    };

    match tera.value_render(template_name, &context) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Tera template '{}': {}", name, e);
            None
        }
    }
}
