extern crate handlebars;

use std::sync::RwLock;

use super::serde::Serialize;
use super::TemplateInfo;

use self::handlebars::Handlebars;
use self::handlebars::HelperDef;

lazy_static! {
    static ref HANDLEBARS: RwLock<Handlebars> = RwLock::new(Handlebars::new());
}

pub const EXT: &'static str = "hbs";

pub fn render<T>(name: &str, info: &TemplateInfo, context: &T) -> Option<String>
    where T: Serialize
{
    // FIXME: Expose a callback to register each template at launch => no lock.
    if HANDLEBARS.read().unwrap().get_template(name).is_none() {
        let p = &info.full_path;
        if let Err(e) = HANDLEBARS.write().unwrap().register_template_file(name, p) {
            error_!("Handlebars template '{}' failed registry: {:?}", name, e);
            return None;
        }
    }

    match HANDLEBARS.read().unwrap().render(name, context) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Handlebars template '{}': {}", name, e);
            None
        }
    }
}

/// Registers a handlebars helper for use with `.hbs` templates
///
/// Define a helper function, exactly as you would for handlebars-rust:
///
/// ```rust,ignore
/// # use handlebars::Context;
/// # use handlebars::Helper;
/// # use handlebars::Handlebars;
/// # use handlebars::RenderContext;
/// # use handlebars::RenderError;
/// fn hex_helper (_: &Context, h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
///     // just for example, add error check for unwrap
///     let param = h.param(0).unwrap().value();
///     let rendered = format!("0x{:x}", param.as_u64().unwrap());
///     try!(rc.writer.write(rendered.into_bytes().as_ref()));
///     Ok(())
/// }
/// ```
///
/// Register it before you ignite (replaces handlebars.register_helper):
///
/// ```rust,ignore
/// rocket_contrib::register_handlebars_helper("hex", Box::new(hex_helper));
/// ```
///
/// Use the helper in your template, as usual:
/// ```
/// {{hex my_value}}
/// ```
pub fn register_handlebars_helper(name: &str,
                       def: Box<HelperDef + 'static>)
                       -> Option<Box<HelperDef + 'static>> {
    let r = HANDLEBARS.write().unwrap().register_helper(name, def);
    r
}
