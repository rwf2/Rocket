//! Dynamic templating engine support for Rocket.
//!
//! This crate adds support for dynamic template rendering to Rocket. It
//! automatically discovers templates, provides a `Responder` to render
//! templates, and automatically reloads templates when compiled in debug mode.
//! At present, it supports [Handlebars] and [Tera].
//!
//! # Usage
//!
//!   1. Depend on `rocket_dyn_templates`. Enable the feature(s) corresponding
//!      to your templating engine(s) of choice:
//!
//!      ```toml
//!      [dependencies.rocket_dyn_templates]
//!      version = "0.1.0"
//!      features = ["handlebars", "tera", "minijinja"]
//!      ```
//!
//!   2. Write your templates inside of the [configurable]
//!      `${ROCKET_ROOT}/templates`. The filename _must_ end with an extension
//!      corresponding to an enabled engine. The second-to-last extension should
//!      correspond to the file's type:
//!
//!      | Engine       | Extension | Example                                    |
//!      |--------------|-----------|--------------------------------------------|
//!      | [Tera]       | `.tera`   | `${ROCKET_ROOT}/templates/index.html.tera` |
//!      | [Handlebars] | `.hbs`    | `${ROCKET_ROOT}/templates/index.html.hbs`  |
//!      | [MiniJinja]  | `.j2`     | `${ROCKET_ROOT}/templates/index.html.j2`   |
//!
//!      [configurable]: #configuration
//!      [Tera]: https://docs.rs/crate/tera/1
//!      [Handlebars]: https://docs.rs/crate/handlebars/6
//!      [MiniJinja]: https://docs.rs/minijinja/2
//!
//!   3. Attach `Template::fairing()` and return a [`Template`] from your routes
//!      via [`Template::render()`], supplying the name of the template file
//!      **minus the last two extensions**:
//!
//!      ```rust
//!      # #[macro_use] extern crate rocket;
//!      use rocket_dyn_templates::{Template, context};
//!
//!      #[get("/")]
//!      fn index() -> Template {
//!          Template::render("index", context! { field: "value" })
//!      }
//!
//!      #[launch]
//!      fn rocket() -> _ {
//!          rocket::build().attach(Template::fairing())
//!      }
//!      ```
//!
//! ## Configuration
//!
//! This crate reads one configuration parameter from the configured figment:
//!
//!   * `template_dir` (**default: `templates/`**)
//!
//!      A path to a directory to search for template files in. Relative paths
//!      are considered relative to the configuration file, or there is no file,
//!      the current working directory.
//!
//! For example, to change the default and set `template_dir` to different
//! values based on whether the application was compiled for debug or release
//! from a `Rocket.toml` file (read by the default figment), you might write:
//!
//! ```toml
//! [debug]
//! template_dir = "static/templates"
//!
//! [release]
//! template_dir = "/var/opt/www/templates"
//! ```
//!
//! **Note:** `template_dir` defaults to `templates/`. It _does not_ need to be
//! specified if the default suffices.
//!
//! See the [configuration chapter] of the guide for more information on
//! configuration.
//!
//! [configuration chapter]: https://rocket.rs/master/guide/configuration
//!
//! ## Template Naming and Content-Types
//!
//! Templates are rendered by _name_ via [`Template::render()`], which returns a
//! [`Template`] responder. The _name_ of the template is the path to the
//! template file, relative to `template_dir`, minus at most two extensions.
//!
//! The `Content-Type` of the response is automatically determined by the
//! non-engine extension using [`ContentType::from_extension()`]. If there is no
//! such extension or it is unknown, `text/plain` is used.
//!
//! The following table contains examples:
//!
//! | template path                                 | [`Template::render()`] call       | content-type |
//! |-----------------------------------------------|-----------------------------------|--------------|
//! | {template_dir}/index.html.hbs                 | `render("index")`                 | HTML         |
//! | {template_dir}/index.tera                     | `render("index")`                 | `text/plain` |
//! | {template_dir}/index.hbs                      | `render("index")`                 | `text/plain` |
//! | {template_dir}/dir/index.hbs                  | `render("dir/index")`             | `text/plain` |
//! | {template_dir}/dir/data.json.tera             | `render("dir/data")`              | JSON         |
//! | {template_dir}/data.template.xml.hbs          | `render("data.template")`         | XML          |
//! | {template_dir}/subdir/index.template.html.hbs | `render("subdir/index.template")` | HTML         |
//!
//! The recommended naming scheme is to use two extensions: one for the file
//! type, and one for the template extension. This means that template
//! extensions should look like: `.html.hbs`, `.html.tera`, `.xml.hbs`, and so
//! on.
//!
//! [`ContentType::from_extension()`]: ../rocket/http/struct.ContentType.html#method.from_extension
//!
//! ### Rendering Context
//!
//! In addition to a name, [`Template::render()`] requires a context to use
//! during rendering. The context can be any [`Serialize`] type that serializes
//! to an `Object` (a dictionary) value. The [`context!`] macro can be used to
//! create inline `Serialize`-able context objects.
//!
//! [`Serialize`]: rocket::serde::Serialize
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::Serialize;
//! use rocket_dyn_templates::{Template, context};
//!
//! #[get("/")]
//! fn index() -> Template {
//!     // Using the `context! { }` macro.
//!     Template::render("index", context! {
//!         site_name: "Rocket - Home Page",
//!         version: 127,
//!     })
//! }
//!
//! #[get("/")]
//! fn index2() -> Template {
//!     #[derive(Serialize)]
//!     #[serde(crate = "rocket::serde")]
//!     struct IndexContext {
//!         site_name: &'static str,
//!         version: u8
//!     }
//!
//!     // Using an existing `IndexContext`, which implements `Serialize`.
//!     Template::render("index", IndexContext {
//!         site_name: "Rocket - Home Page",
//!         version: 127,
//!     })
//! }
//! ```
//!
//! ### Discovery, Automatic Reloads, and Engine Customization
//!
//! As long as one of [`Template::fairing()`], [`Template::custom()`], or
//! [`Template::try_custom()`] is [attached], any file in the configured
//! `template_dir` ending with a known engine extension (as described in the
//! [usage section](#usage)) can be rendered. The latter two fairings allow
//! customizations such as registering helpers and templates from strings.
//!
//! _**Note:** Templates that are registered directly via [`Template::custom()`],
//! use whatever name provided during that registration; no extensions are
//! automatically removed._
//!
//! In debug mode (without the `--release` flag passed to `cargo`), templates
//! are **automatically reloaded** from disk when changes are made. In release
//! builds, template reloading is disabled to improve performance and cannot be
//! enabled.
//!
//! [attached]: rocket::Rocket::attach()
//!
//! ### Metadata and Rendering to `String`
//!
//! The [`Metadata`] request guard allows dynamically querying templating
//! metadata, such as whether a template is known to exist
//! ([`Metadata::contains_template()`]), and to render templates to `String`
//! ([`Metadata::render()`]).

#![doc(html_root_url = "https://api.rocket.rs/master/rocket_dyn_templates")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

#[macro_use] extern crate rocket;

#[doc(inline)]
#[cfg(feature = "tera")]
/// The tera templating engine library, reexported.
pub use tera;

#[doc(inline)]
#[cfg(feature = "handlebars")]
/// The handlebars templating engine library, reexported.
pub use handlebars;

#[doc(inline)]
#[cfg(feature = "minijinja")]
/// The minijinja templating engine library, reexported.
pub use minijinja;

#[doc(hidden)]
pub use rocket::serde;

mod engine;
mod fairing;
mod context;
mod metadata;
mod template;

pub use engine::Engines;
pub use metadata::Metadata;
pub use template::Template;
