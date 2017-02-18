extern crate tera;

use super::serde::Serialize;
use super::TemplateInfo;

use self::tera::Tera;

static mut TERA: Option<Tera> = None;

pub const EXT: &'static str = "tera";

// This function must be called a SINGLE TIME from A SINGLE THREAD for safety to
// hold here and in `render`.
pub unsafe fn register(templates: &[(&str, &TemplateInfo)]) -> bool {
    if TERA.is_some() {
        error_!("Internal error: reregistering Tera!");
        return false;
    }

    let mut tera = Tera::default();
    let ext = [".html.tera", ".htm.tera", ".xml.tera", ".html", ".htm", ".xml"];
    tera.autoescape_on(ext.to_vec());

    // Collect into a tuple of (name, path) for Tera.
    let tera_templates = templates.iter()
        .map(|&(name, info)| (&info.full_path, Some(name)))
        .collect::<Vec<_>>();

    // Finally try to tell Tera about all of the templates.
    let mut success = true;
    if let Err(e) = tera.add_template_files(tera_templates) {
        error_!("Failed to initialize Tera templates: {:?}", e);
        success = false;
    }

    TERA = Some(tera);
    success
}

pub fn render<T>(name: &str, _: &TemplateInfo, context: &T) -> Option<String>
    where T: Serialize
{
    let tera = match unsafe { TERA.as_ref() } {
        Some(tera) => tera,
        None => {
            error_!("Internal error: `render` called before Tera init.");
            return None;
        }
    };

    if tera.get_template(name).is_err() {
        error_!("Tera template '{}' does not exist.", name);
        return None;
    };

    match tera.value_render(name, context) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Tera template '{}'.", name);
            for error in e.iter().skip(1) {
                error_!("{}.", error);
            }

            None
        }
    }
}
