#[macro_use] extern crate rocket;

use std::path::{Path, PathBuf};

use rocket::{Rocket, Build};
use rocket::config::Config;
use rocket::figment::value::Value;
use rocket::serde::{Serialize, Deserialize};
use rocket_dyn_templates::{Template, Metadata, context};

#[get("/<engine>/<name>")]
fn template_check(md: Metadata<'_>, engine: &str, name: &str) -> Option<()> {
    match md.contains_template(&format!("{}/{}", engine, name)) {
        true => Some(()),
        false => None
    }
}

#[get("/is_reloading")]
fn is_reloading(md: Metadata<'_>) -> Option<()> {
    if md.reloading() { Some(()) } else { None }
}

fn template_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("templates")
}

fn rocket() -> Rocket<Build> {
    rocket::custom(Config::figment().merge(("template_dir", template_root())))
        .attach(Template::fairing())
        .mount("/", routes![template_check, is_reloading])
}

#[test]
fn test_callback_error() {
    use rocket::{local::blocking::Client, error::ErrorKind::FailedFairings};

    let rocket = rocket::build().attach(Template::try_custom(|_| {
        Err("error reloading templates!".into())
    }));

    let error = Client::debug(rocket).expect_err("client failure");
    match error.kind() {
        FailedFairings(failures) => assert_eq!(failures[0].name, "Templating"),
        _ => panic!("Wrong kind of launch error"),
    }
}

#[test]
fn test_sentinel() {
    use rocket::{local::blocking::Client, error::ErrorKind::SentinelAborts};

    let err = Client::debug_with(routes![is_reloading]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    let err = Client::debug_with(routes![is_reloading, template_check]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 2));

    #[get("/")]
    fn return_template() -> Template {
        Template::render("foo", ())
    }

    let err = Client::debug_with(routes![return_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    #[get("/")]
    fn return_opt_template() -> Option<Template> {
        Some(Template::render("foo", ()))
    }

    let err = Client::debug_with(routes![return_opt_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    #[derive(rocket::Responder)]
    struct MyThing<T>(T);

    #[get("/")]
    fn return_custom_template() -> MyThing<Template> {
        MyThing(Template::render("foo", ()))
    }

    let err = Client::debug_with(routes![return_custom_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    #[derive(rocket::Responder)]
    struct MyOkayThing<T>(Option<T>);

    impl<T> rocket::Sentinel for MyOkayThing<T> {
        fn abort(_: &Rocket<rocket::Ignite>) -> bool {
            false
        }
    }

    #[get("/")]
    fn always_ok_sentinel() -> MyOkayThing<Template> {
        MyOkayThing(None)
    }

    Client::debug_with(routes![always_ok_sentinel]).expect("no sentinel abort");
}

#[test]
fn test_context_macro() {
    macro_rules! assert_same_object {
        ($ctx:expr, $obj:expr $(,)?) => {{
            let ser_ctx = Value::serialize(&$ctx).unwrap();
            let deser_ctx = ser_ctx.deserialize().unwrap();
            assert_eq!($obj, deser_ctx);
        }};
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Empty { }

        assert_same_object!(context! { }, Empty { });
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Object {
            a: u32,
            b: String,
        }

        let a = 93;
        let b = "Hello".to_string();

        fn make_context() -> impl Serialize {
            let b = "Hello".to_string();

            context! { a: 93, b: b }
        }

        assert_same_object!(
            make_context(),
            Object { a, b },
        );
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Outer {
            s: String,
            inner: Inner,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Inner {
            center: Center,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Center {
            value_a: bool,
            value_b: u8,
        }

        let a = true;
        let value_b = 123;
        let outer_string = String::from("abc 123");

        assert_same_object!(
            context! {
                s: &outer_string,
                inner: context! {
                    center: context! {
                        value_a: a,
                        value_b,
                    },
                },
            },
            Outer {
                s: outer_string,
                inner: Inner {
                    center: Center {
                        value_a: a,
                        value_b,
                    },
                },
            },
        );
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Object {
            a: String,
        }

        let owned = String::from("foo");
        let ctx = context! { a: &owned };
        assert_same_object!(ctx, Object { a: "foo".into() });
        drop(ctx);
        drop(owned);
    }
}

#[cfg(feature = "tera")]
mod tera_tests {
    use super::*;
    use std::collections::HashMap;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    const UNESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot\n";
    const ESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script &#x2F;&gt;\n\nfoot\n";

    #[test]
    fn test_tera_templates() {
        let client = Client::debug(rocket()).unwrap();
        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(client.rocket(), "tera/txt_test", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));

        // Now with an HTML file, which should.
        let template = Template::show(client.rocket(), "tera/html_test", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
    }

    // u128 is not supported. enable when it is.
    // #[test]
    // fn test_tera_u128() {
    //     const EXPECTED: &'static str
    //         = "\nh_start\ntitle: 123\nh_end\n\n\n1208925819614629174706176\n\nfoot\n";
    //
    //     let client = Client::debug(rocket()).unwrap();
    //     let mut map = HashMap::new();
    //     map.insert("title", 123);
    //     map.insert("number", 1u128 << 80);
    //
    //     let template = Template::show(client.rocket(), "tera/txt_test", &map);
    //     assert_eq!(template, Some(EXPECTED.into()));
    // }

    #[test]
    fn test_template_metadata_with_tera() {
        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/tera/txt_test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/tera/html_test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/tera/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let response = client.get("/hbs/txt_test").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}

#[cfg(feature = "handlebars")]
mod handlebars_tests {
    use super::*;
    use std::collections::HashMap;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn test_handlebars_templates() {
        const EXPECTED: &'static str
            = "Hello _test_!\n\n<main> &lt;script /&gt; hi </main>\nDone.\n\n";

        let client = Client::debug(rocket()).unwrap();
        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script /> hi");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(client.rocket(), "hbs/test", &map);
        assert_eq!(template, Some(EXPECTED.into()));
    }

    // u128 is not supported. enable when it is.
    // #[test]
    // fn test_handlebars_u128() {
    //     const EXPECTED: &'static str
    //         = "Hello 123!\n\n<main> 1208925819614629174706176 </main>\nDone.\n\n";
    //
    //     let client = Client::debug(rocket()).unwrap();
    //     let mut map = HashMap::new();
    //     map.insert("title", 123);
    //     map.insert("number", 1u128 << 80);
    //
    //     let template = Template::show(client.rocket(), "hbs/test", &map);
    //     assert_eq!(template, Some(EXPECTED.into()));
    // }

    #[test]
    fn test_template_metadata_with_handlebars() {
        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/hbs/test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/hbs/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let response = client.get("/tera/test").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_template_reload() {
        use std::fs::File;
        use std::io::Write;
        use std::time::Duration;

        use rocket::local::blocking::Client;

        const RELOAD_TEMPLATE: &str = "hbs/reload";
        const INITIAL_TEXT: &str = "initial";
        const NEW_TEXT: &str = "reload";

        fn write_file(path: &Path, text: &str) {
            let mut file = File::create(path).expect("open file");
            file.write_all(text.as_bytes()).expect("write file");
            file.sync_all().expect("sync file");
        }

        // set up the template before initializing the Rocket instance so
        // that it will be picked up in the initial loading of templates.
        let reload_path = template_root().join("hbs").join("reload.txt.hbs");
        write_file(&reload_path, INITIAL_TEXT);

        // set up the client. if we can't reload templates, then just quit
        let client = Client::debug(rocket()).unwrap();
        let res = client.get("/is_reloading").dispatch();
        if res.status() != Status::Ok {
            return;
        }

        // verify that the initial content is correct
        let initial_rendered = Template::show(client.rocket(), RELOAD_TEMPLATE, ());
        assert_eq!(initial_rendered, Some(INITIAL_TEXT.into()));

        // write a change to the file
        write_file(&reload_path, NEW_TEXT);

        for _ in 0..6 {
            // dispatch any request to trigger a template reload
            client.get("/").dispatch();

            // if the new content is correct, we are done
            let new_rendered = Template::show(client.rocket(), RELOAD_TEMPLATE, ());
            if new_rendered == Some(NEW_TEXT.into()) {
                write_file(&reload_path, INITIAL_TEXT);
                return;
            }

            // otherwise, retry a few times, waiting 250ms in between
            std::thread::sleep(Duration::from_millis(250));
        }

        panic!("failed to reload modified template in 1.5s");
    }
}
