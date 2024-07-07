use rocket::response::Redirect;
use rocket::Request;

use rocket_dyn_templates::{context, minijinja::Environment, Template};

// use self::minijinja::;

#[get("/")]
pub fn index() -> Redirect {
    Redirect::to(uri!("/minijinja", hello(name = "Your Name")))
}

#[get("/hello/<name>")]
pub fn hello(name: &str) -> Template {
    Template::render(
        "minijinja/index",
        context! {
            title: "Hello",
            name: Some(name),
            items: vec!["One", "Two", "Three"],
        },
    )
}

#[get("/about")]
pub fn about() -> Template {
    Template::render(
        "minijinja/about.html",
        context! {
            title: "About",
        },
    )
}

#[catch(404)]
pub fn not_found(req: &Request<'_>) -> Template {
    println!("Handling 404 for URI: {}", req.uri());

    Template::render(
        "minijinja/error/404",
        context! {
            uri: req.uri()
        },
    )
}

pub fn customize(env: &mut Environment) {
    env.add_template(
        "minijinja/about.html",
        r#"
        {% extends "minijinja/layout" %}

        {% block page %}
            <section id="about">
                <h1>About - Here's another page!</h1>
            </section>
        {% endblock %}
    "#,
    )
    .expect("valid Jinja2 template");
}
