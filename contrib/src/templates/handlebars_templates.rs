extern crate handlebars;

use super::serde::Serialize;
use super::TemplateInfo;

use self::handlebars::Handlebars;

static mut HANDLEBARS: Option<Handlebars> = None;

pub const EXT: &'static str = "hbs";

// This function must be called a SINGLE TIME from A SINGLE THREAD for safety to
// hold here and in `render`.
pub unsafe fn register(templates: &[(&str, &TemplateInfo)]) -> bool {
    if HANDLEBARS.is_some() {
        error_!("Internal error: reregistering handlebars!");
        return false;
    }

    let mut hb = Handlebars::new();
    let mut success = true;
    for &(name, info) in templates {
        let path = &info.full_path;
        if let Err(e) = hb.register_template_file(name, path) {
            error_!("Handlebars template '{}' failed registry: {:?}", name, e);
            success = false;
        }
    }

    HANDLEBARS = Some(hb);
    success
}

pub fn render<T>(name: &str, _info: &TemplateInfo, context: &T) -> Option<String>
    where T: Serialize
{
    let hb = match unsafe { HANDLEBARS.as_ref() } {
        Some(hb) => hb,
        None => {
            error_!("Internal error: `render` called before handlebars init.");
            return None;
        }
    };

    if hb.get_template(name).is_none() {
        error_!("Handlebars template '{}' does not exist.", name);
        return None;
    }

    match hb.render(name, context) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Handlebars template '{}': {}", name, e);
            None
        }
    }
}
