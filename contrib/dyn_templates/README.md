# `dyn_templates` [![ci.svg]][ci] [![crates.io]][crate] [![docs.svg]][crate docs]

[crates.io]: https://img.shields.io/crates/v/rocket_dyn_templates.svg
[crate]: https://crates.io/crates/rocket_dyn_templates
[docs.svg]: https://img.shields.io/badge/web-master-red.svg?style=flat&label=docs&colorB=d33847
[crate docs]: https://api.rocket.rs/master/rocket_dyn_templates
[ci.svg]: https://github.com/rwf2/Rocket/workflows/CI/badge.svg
[ci]: https://github.com/rwf2/Rocket/actions

This crate adds support for dynamic template rendering to Rocket. It
automatically discovers templates, provides a `Responder` to render templates,
and automatically reloads templates when compiled in debug mode. At present, it
supports [Handlebars], [Tera], and [MiniJinja].

[Tera]: https://docs.rs/crate/tera/2
[Handlebars]: https://docs.rs/crate/handlebars/6
[MiniJinja]: https://docs.rs/minijinja/3

# Usage

  1. Enable the `rocket_dyn_templates` feature corresponding to your templating
     engine(s) of choice:

     ```toml
     [dependencies.rocket_dyn_templates]
     version = "0.1.0"
     features = ["handlebars", "tera", "minijinja"]
     ```

  1. Write your template files in Handlebars (`.hbs`), Tera (`.tera`), or
     MiniJinja (`.j2`) in the configurable `template_dir` directory (default:
     `{rocket_root}/templates`).

  2. Attach `Templating::new()` and return a `Template` using
     `Template::render()`, supplying the name of the template file **minus the
     engine extension**:

     ```rust
     use rocket_dyn_templates::{Template, Templating, context};

     #[get("/")]
     fn index() -> Template {
         Template::render("template-name.html", context! { field: "value" })
     }

     #[launch]
     fn rocket() -> _ {
         rocket::build().attach(Templating::new())
     }
     ```

See the [crate docs] for full details.
