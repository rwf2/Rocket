// Without an engine, the shared test helpers are unused.
#![cfg_attr(
    not(any(feature = "tera", feature = "handlebars", feature = "minijinja")),
    allow(dead_code)
)]

#[macro_use] extern crate rocket;

use std::path::{Path, PathBuf};

use rocket::{Rocket, Build};
use rocket::config::Config;
use rocket::figment::value::Value;
use rocket::serde::{Serialize, Deserialize};
use rocket_dyn_templates::{Template, Templating, Metadata, context};

#[get("/<engine>/<name>")]
fn template_check(md: Metadata<'_>, engine: &str, name: &str) -> Option<()> {
    md.contains_template(&format!("{}/{}", engine, name)).then(|| ())
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
        .attach(Templating::new())
        .mount("/", routes![template_check, is_reloading])
}

#[test]
fn test_callback_error() {
    use rocket::{local::blocking::Client, error::ErrorKind::FailedFairings};

    let dir = tempfile::tempdir().expect("template directory");
    for templating in [
        Templating::try_init(|_| Err("error initializing templates!".into()))
            .on_load(|_| panic!("loading after failed initialization")),
        Templating::new()
            .try_on_load(|_| Err("error loading templates!".into())),
    ] {
        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let error = Client::debug(rocket).expect_err("client failure");
        match error.kind() {
            FailedFairings(failures) => assert_eq!(failures[0].name, "Templating"),
            _ => panic!("Wrong kind of launch error"),
        }
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
    use rocket::http::{ContentType, Status};
    use rocket::request::FromRequest;
    use pretty_assertions::assert_eq;

    const UNESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot";
    const ESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script &#x2F;&gt;\n\nfoot";

    #[async_test]
    async fn test_tera_templates() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(client.rocket(), "tera/txt_test", &map);
        let md_rendered = metadata.render("tera/txt_test", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::Text, UNESCAPED_EXPECTED.into())));

        // Now with an HTML file, which should escape.
        let template = Template::show(client.rocket(), "tera/html_test", &map);
        let md_rendered = metadata.render("tera/html_test", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::HTML, ESCAPED_EXPECTED.into())));
    }

    #[async_test]
    async fn test_globby_paths() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        assert!(metadata.contains_template("tera/[test]/html_test"));
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
        use rocket::local::blocking::Client;

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
    use rocket::request::FromRequest;
    use rocket::http::{ContentType, Status};
    use pretty_assertions::assert_eq;

    #[async_test]
    async fn test_handlebars_templates() {
        use rocket::local::asynchronous::Client;

        const EXPECTED: &'static str
            = "Hello _test_!\n<main> &lt;script /&gt; hi </main>\nDone.\n";

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script /> hi");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(client.rocket(), "hbs/test", &map);
        let md_rendered = metadata.render("hbs/test", &map);
        assert_eq!(template, Some(EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::HTML, EXPECTED.into())));
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
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/hbs/test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/hbs/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let response = client.get("/tera/test").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}

#[cfg(feature = "minijinja")]
mod j2_tests {
    use super::*;
    use std::collections::HashMap;
    use rocket::http::{ContentType, Status};
    use rocket::request::FromRequest;
    use pretty_assertions::assert_eq;

    const UNESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot";
    const ESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script &#x2f;&gt;\n\nfoot";

    #[async_test]
    async fn test_j2_templates() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(client.rocket(), "j2/txt_test", &map);
        let md_rendered = (&metadata).render("j2/txt_test", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::Text, UNESCAPED_EXPECTED.into())));

        // Now with an HTML file, which should escaped.
        let template = Template::show(client.rocket(), "j2/html_test", &map);
        let md_rendered = metadata.render("j2/html_test", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::HTML, ESCAPED_EXPECTED.into())));
    }

    #[async_test]
    async fn test_globby_paths() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        assert!(metadata.contains_template("j2/[test]/html_test"));
    }

    #[test]
    fn test_j2_u128() {
        const EXPECTED: &'static str
            = "\nh_start\ntitle: 123\nh_end\n\n\n1208925819614629174706176\n\nfoot";

        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();
        let mut map = HashMap::new();
        map.insert("title", 123);
        map.insert("content", 1u128 << 80);

        let template = Template::show(client.rocket(), "j2/txt_test", &map);
        assert_eq!(template, Some(EXPECTED.into()));
    }

    #[test]
    fn test_template_metadata_with_j2() {
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/j2/txt_test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/j2/html_test").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/j2/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let response = client.get("/hbs/txt_test").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}

#[test]
#[cfg(debug_assertions)]
fn test_template_reload() {
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use rocket::http::Status;
    use rocket::local::blocking::Client;

    fn write_file(path: &Path, text: &str) {
        let mut file = File::create(path).expect("open file");
        file.write_all(text.as_bytes()).expect("write file");
        file.sync_all().expect("sync file");
    }

    fn wait_for(client: &Client, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            // Dispatch any request to trigger a template reload.
            client.get("/").dispatch();
            if condition() {
                return;
            }

            assert!(Instant::now() < deadline, "failed to reload templates in 3s");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    let engines: &[(&str, &str)] = &[
        #[cfg(feature = "handlebars")]
        ("hbs", "{{#if}}"),
        #[cfg(feature = "tera")]
        ("tera", "{% if %}"),
        #[cfg(feature = "minijinja")]
        ("j2", "{% if %}"),
    ];

    for &(ext, invalid) in engines {
        let dir = tempfile::tempdir().expect("template directory");
        let path = dir.path().join(format!("reload.txt.{ext}"));
        let renamed = dir.path().join(format!("renamed.txt.{ext}"));

        // Set up the template before initializing Rocket so that it will be
        // picked up in the initial loading of templates.
        write_file(&path, "initial");
        let callback_path = path.clone();
        let (failed, failure) = mpsc::channel();
        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(Templating::new().try_on_load(move |_| {
                if matches!(fs::read_to_string(&callback_path), Ok(text) if text == "reject") {
                    failed.send(()).unwrap();
                    return Err("reject reload".into());
                }

                Ok(())
            }))
            .mount("/", routes![is_reloading]);
        let client = Client::debug(rocket).unwrap();
        assert_eq!(client.get("/is_reloading").dispatch().status(), Status::Ok);

        let render = |name| Template::show(client.rocket(), name, context! {});
        assert_eq!(render("reload"), Some("initial".into()));

        // Write a change and wait for the new content to be rendered.
        write_file(&path, "reload");
        wait_for(&client, || render("reload") == Some("reload".into()));

        // A rejected reload keeps the previous templates active.
        write_file(&path, "reject");
        wait_for(&client, || failure.try_recv().is_ok());
        assert_eq!(render("reload"), Some("reload".into()));

        if ext == "j2" {
            // MiniJinja parses lazily, so invalid edits fail to render.
            write_file(&path, invalid);
            wait_for(&client, || render("reload").is_none());
        }

        write_file(&path, "repaired");
        wait_for(&client, || render("reload") == Some("repaired".into()));

        fs::rename(&path, &renamed).expect("rename template");
        wait_for(&client, || render("renamed") == Some("repaired".into()));
        assert_eq!(render("reload"), None);

        fs::remove_file(&renamed).expect("remove template");
        wait_for(&client, || render("renamed").is_none());

        // Dropping the client also stops the watcher before removing its root.
        drop(client);
        dir.close().expect("remove template directory");
    }
}
