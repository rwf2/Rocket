extern crate serde;
extern crate serde_json;
extern crate glob;

#[cfg(feature = "tera_templates")]
mod tera_templates;

#[cfg(feature = "handlebars_templates")]
mod handlebars_templates;

#[macro_use] mod macros;

use self::serde::Serialize;
use self::glob::glob;

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use rocket::config;
use rocket::response::{self, Content, Responder};
use rocket::http::{ContentType, Status};

/// The Template type implements generic support for template rendering in
/// Rocket.
///
/// Templating in Rocket words by first discovering all of the templates inside
/// the template directory. The template directory is configurable via the
/// `template_dir` configuration parameter and defaults to `templates/`. The
/// path set in `template_dir` should be relative to the Rocket configuration
/// file. See the [configuration chapter](https://rocket.rs/guide/overview/#configuration)
/// of the guide for more information on configuration.
///
/// Templates are discovered according to their extension. At present, this
/// library supports the following templates and extensions:
///
/// * **Tera**: `.tera`
/// * **Handlebars**: `.hbs`
///
/// Any file that ends with one of these extension will be discovered and
/// rendered with the corresponding templating engine. The name of the template
/// will be the path to the template file relative to `template_dir` minus at
/// most two extensions. The following are examples of template names (on the
/// right) given that the template is at the path on the left.
///
///   * `{template_dir}/index.html.hbs` => index
///   * `{template_dir}/index.tera` => index
///   * `{template_dir}/index.hbs` => index
///   * `{template_dir}/dir/index.hbs` => dir/index
///   * `{template_dir}/dir/index.html.tera` => dir/index
///   * `{template_dir}/index.template.html.hbs` => index.template
///   * `{template_dir}/subdir/index.template.html.hbs` => subdir/index.template
///
/// The recommended naming scheme is to use two extensions: one for the file
/// type, and one for the template extension. This means that template
/// extensions should look like: `.html.hbs`, `.html.tera`, `.xml.hbs`, etc.
///
/// Templates are rendered with the `render` method. The method takes in the
/// name of a template and a context to render the template with. The context
/// can be any type that implements `Serialize` from
/// [Serde](https://github.com/serde-rs/json) and would serialize to an `Object`
/// value.
///
/// # Usage
///
/// To use, add the `handlebars_templates` feature, the `tera_templates`
/// feature, or both, to the `rocket_contrib` dependencies section of your
/// `Cargo.toml`:
///
/// ```toml,ignore
/// [dependencies.rocket_contrib]
/// version = "*"
/// default-features = false
/// features = ["handlebars_templates", "tera_templates"]
/// ```
///
/// # Examples
///
/// To render a template named "index" with a `HashMap` as the context:
///
/// ```rust
/// use rocket_contrib::Template;
/// use std::collections::HashMap;
///
/// let context: HashMap<&str, &str> = HashMap::new();
/// // ... add key/value pairs to `context` ...
/// let _template = Template::render("index", &context);
/// ```
///
/// The Template type implements Rocket's `Responder` trait, so it can be
/// returned from a request handler directly:
///
/// ```rust,ignore
/// #[get("/")]
/// fn index() -> Template {
///     let context = ...;
///     Template::render("index", &context)
/// }
/// ```
#[derive(Debug)]
pub struct Template(Option<String>, Option<String>);

#[derive(Debug)]
pub struct TemplateInfo {
    /// The complete path, including `template_dir`, to this template.
    full_path: PathBuf,
    /// The complete path, without `template_dir`, to this template.
    path: PathBuf,
    /// The extension for the engine of this template.
    extension: String,
    /// The extension before the engine extension in the template, if any.
    data_type: Option<String>
}

const DEFAULT_TEMPLATE_DIR: &'static str = "templates";

lazy_static! {
    static ref TEMPLATES: HashMap<String, TemplateInfo> = discover_templates();
    static ref TEMPLATE_DIR: String = {
        config::active().map(|config| {
            let dir = config.get_str("template_dir").map_err(|e| {
                if !e.is_not_found() {
                    e.pretty_print();
                    warn_!("Using default directory '{}'", DEFAULT_TEMPLATE_DIR);
                }
            }).unwrap_or(DEFAULT_TEMPLATE_DIR);

            config.root().join(dir).to_string_lossy().into_owned()
        }).unwrap_or(DEFAULT_TEMPLATE_DIR.to_string())
    };
}

impl Template {
    /// Render the template named `name` with the context `context`. The
    /// template is not actually rendered until the response is needed by
    /// Rocket. As such, the `Template` type should be used only as a
    /// `Responder`.
    pub fn render<S, T>(name: S, context: &T) -> Template
        where S: AsRef<str>, T: Serialize
    {
        let name = name.as_ref();
        let template = TEMPLATES.get(name);
        if template.is_none() {
            let names: Vec<_> = TEMPLATES.keys().map(|s| s.as_str()).collect();
            error_!("Template '{}' does not exist.", name);
            info_!("Known templates: {}", names.join(","));
            info_!("Searched in '{}'.", *TEMPLATE_DIR);
            return Template(None, None);
        }

        // Keep this set in-sync with the `engine_set` invocation.
        render_set!(name, template.unwrap(), context,
            "tera_templates" => tera_templates,
            "handlebars_templates" => handlebars_templates,
        );

        unreachable!("A template extension was discovered but not rendered.")
    }
}

impl Responder<'static> for Template {
    fn respond(self) -> response::Result<'static> {
        let content_type = match self.1 {
            Some(ref ext) => ContentType::from_extension(ext),
            None => ContentType::HTML
        };

        match self.0 {
            Some(render) => Content(content_type, render).respond(),
            None => Err(Status::InternalServerError)
        }
    }
}

/// Removes the file path's extension or does nothing if there is none.
fn remove_extension<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    let stem = match path.file_stem() {
        Some(stem) => stem,
        None => return path.to_path_buf()
    };

    match path.parent() {
        Some(parent) => parent.join(stem),
        None => PathBuf::from(stem)
    }
}

/// Returns a HashMap of `TemplateInfo`'s for all of the templates in
/// `TEMPLATE_DIR`. Templates are all files that match one of the extensions for
/// engine's in `engine_set`.
fn discover_templates() -> HashMap<String, TemplateInfo> {
    // Keep this set in-sync with the `render_set` invocation.
    let engines = engine_set![
        "tera_templates" => tera_templates,
        "handlebars_templates" => handlebars_templates,
    ];

    let mut templates = HashMap::new();
    for ext in engines {
        let mut glob_path: PathBuf = [&*TEMPLATE_DIR, "**", "*"].iter().collect();
        glob_path.set_extension(ext);
        for path in glob(glob_path.to_str().unwrap()).unwrap().filter_map(Result::ok) {
            let rel_path = path.strip_prefix(&*TEMPLATE_DIR).unwrap().to_path_buf();
            let path_no_ext = remove_extension(&rel_path);
            let data_type = path_no_ext.extension();
            let name = remove_extension(&path_no_ext);
            templates.insert(name.to_string_lossy().into_owned(), TemplateInfo {
                full_path: path.to_path_buf(),
                path: rel_path,
                extension: path.extension().unwrap().to_string_lossy().into_owned(),
                data_type: data_type.map(|d| d.to_string_lossy().into_owned())
            });
        }
    }

    templates
}

