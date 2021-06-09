# `dyn_templates` [![ci.svg]][ci] [![crates.io]][crate] [![docs.svg]][crate docs]

[crates.io]: https://img.shields.io/crates/v/rocket_dyn_templates.svg
[crate]: https://crates.io/crates/rocket_dyn_templates
[docs.svg]: https://img.shields.io/badge/web-master-red.svg?style=flat&label=docs&colorB=d33847
[crate docs]: https://api.rocket.rs/v0.5-rc/rocket_dyn_templates
[ci.svg]: https://github.com/SergioBenitez/Rocket/workflows/CI/badge.svg
[ci]: https://github.com/SergioBenitez/Rocket/actions

This crate adds support for dynamic template rendering to Rocket. It
automatically discovers templates, provides a `Responder` to render templates,
and automatically reloads templates when compiled in debug mode. At present, it
supports [Handlebars] and [Tera].

[Tera]: https://docs.rs/crate/tera/1
[Handlebars]: https://docs.rs/crate/handlebars/3

# Usage

  1. Enable the `rocket_dyn_templates` feature corresponding to your templating
     engine(s) of choice:

     ```toml
     [dependencies.rocket_dyn_templates]
     version = "0.1.0-rc.1"
     features = ["handlebars", "tera"]
     ```

  1. Write your template files in Handlebars (`.hbs`) and/or Tera (`.tera`) in
     the configurable `template_dir` directory (default:
     `{rocket_root}/templates`).

  2. Attach `Template::fairing()` and return a `Template` using
     `Template::render()`, supplying the name of the template file **minus the
     last two extensions**:

     ```rust
     use rocket_dyn_templates::Template;

     #[launch]
     fn rocket() -> _ {
         rocket::build().attach(Template::fairing())
     }

     #[get("/")]
     fn index() -> Template {
         Template::render("template-name", &context)
     }
     ```

See the [crate docs] for full details.
