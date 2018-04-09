use std::path::{Path, PathBuf};
use std::collections::HashMap;

use super::{Engines, TemplateInfo};
#[cfg(debug_assertions)]
use super::watch::TemplateWatcher;
use super::glob;

use rocket::http::ContentType;

pub struct Context {
    /// The root of the template directory.
    pub root: PathBuf,
    /// Mapping from template name to its information.
    pub templates: HashMap<String, TemplateInfo>,
    /// Loaded template engines, or None if there was a failure during a reload
    pub engines: Option<Engines>,
    /// Context customization callback for reuse when reloading
    pub customize_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>,
    /// Filesystem watcher, or None if the directory could not be watched
    #[cfg(debug_assertions)]
    pub watcher: Option<TemplateWatcher>,
}

impl Context {
    pub fn initialize(
        root: PathBuf,
        customize_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>,
    ) -> Option<Context> {
        let mut ctxt = Context {
            root: root.clone(),
            templates: Default::default(),
            engines: Default::default(),
            customize_callback: customize_callback,
            #[cfg(debug_assertions)]
            watcher: TemplateWatcher::new(root),
        };

        ctxt.reload();
        match ctxt.engines {
            Some(_) => Some(ctxt),
            _ => None,
        }
    }

    pub fn reload(&mut self) {
        let mut templates: HashMap<String, TemplateInfo> = HashMap::new();
        for ext in Engines::ENABLED_EXTENSIONS {
            let mut glob_path = self.root.join("**").join("*");
            glob_path.set_extension(ext);
            let glob_path = glob_path.to_str().expect("valid glob path string");

            for path in glob(glob_path).unwrap().filter_map(Result::ok) {
                let (name, data_type_str) = split_path(&self.root, &path);
                if let Some(info) = templates.get(&*name) {
                    warn_!("Template name '{}' does not have a unique path.", name);
                    info_!("Existing path: {:?}", info.path);
                    info_!("Additional path: {:?}", path);
                    warn_!("Using existing path for template '{}'.", name);
                    continue;
                }

                let data_type = data_type_str.as_ref()
                    .and_then(|ext| ContentType::from_extension(ext))
                    .unwrap_or(ContentType::HTML);

                templates.insert(name, TemplateInfo {
                    path: path.to_path_buf(),
                    extension: ext.to_string(),
                    data_type: data_type,
                });
            }
        }

        let engines = Engines::init(&templates).map(|mut engines| {
            (self.customize_callback)(&mut engines);
            engines
        });

        self.templates = templates;
        self.engines = engines;
    }

    #[cfg(debug_assertions)]
    pub fn reload_if_needed(&mut self) {
        if self.watcher.as_ref().map(TemplateWatcher::needs_reload).unwrap_or(false) {
            warn!("Change detected, reloading templates");
            self.reload();
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

/// Splits a path into a name that may be used to identify the template, and the
/// template's data type, if any.
fn split_path(root: &Path, path: &Path) -> (String, Option<String>) {
    let rel_path = path.strip_prefix(root).unwrap().to_path_buf();
    let path_no_ext = remove_extension(&rel_path);
    let data_type = path_no_ext.extension();
    let mut name = remove_extension(&path_no_ext).to_string_lossy().into_owned();

    // Ensure template name consistency on Windows systems
    if cfg!(windows) {
        name = name.replace("\\", "/");
    }

    (name, data_type.map(|d| d.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_path_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            for filename in &["index.html.hbs", "index.html.tera"] {
                let path = Path::new(root).join(filename);
                let (name, data_type) = split_path(Path::new(root), &path);

                assert_eq!(name, "index");
                assert_eq!(data_type, Some("html".into()));
            }
        }
    }

    #[test]
    fn template_path_subdir_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            for sub in &["a/", "a/b/", "a/b/c/", "a/b/c/d/"] {
                for filename in &["index.html.hbs", "index.html.tera"] {
                    let path = Path::new(root).join(sub).join(filename);
                    let (name, data_type) = split_path(Path::new(root), &path);

                    let expected_name = format!("{}index", sub);
                    assert_eq!(name, expected_name.as_str());
                    assert_eq!(data_type, Some("html".into()));
                }
            }
        }
    }

    #[test]
    fn template_path_doc_examples() {
        fn name_for(path: &str) -> String {
            split_path(Path::new("templates/"), &Path::new("templates/").join(path)).0
        }

        assert_eq!(name_for("index.html.hbs"), "index");
        assert_eq!(name_for("index.tera"), "index");
        assert_eq!(name_for("index.hbs"), "index");
        assert_eq!(name_for("dir/index.hbs"), "dir/index");
        assert_eq!(name_for("dir/index.html.tera"), "dir/index");
        assert_eq!(name_for("index.template.html.hbs"), "index.template");
        assert_eq!(name_for("subdir/index.template.html.hbs"), "subdir/index.template");
    }
}
