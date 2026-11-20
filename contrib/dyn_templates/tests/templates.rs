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

#[cfg(any(feature = "tera", feature = "minijinja"))]
#[async_test]
async fn test_template_names_and_escaping() {
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use rocket::request::FromRequest;

    let mut cases = Vec::new();
    for (suffix, content_type) in [
        ("html", ContentType::HTML),
        ("htm", ContentType::HTML),
        ("xml", ContentType::XML),
        ("xhtml", ContentType::XHTML),
        ("svg", ContentType::SVG),
    ] {
        for mask in 0..1 << suffix.len() {
            let extension: String = suffix.bytes().enumerate()
                .map(|(i, byte)| if mask & (1 << i) == 0 {
                    byte as char
                } else {
                    byte.to_ascii_uppercase() as char
                })
                .collect();

            let name = format!("case_{suffix}_{mask}.{extension}");
            cases.push((name, content_type.clone(), true));
        }
    }

    cases.extend([
        ("plain".into(), ContentType::Text, false),
        ("html".into(), ContentType::Text, false),
        ("html.j2".into(), ContentType::Text, false),
        ("same.html".into(), ContentType::HTML, true),
        ("same.txt".into(), ContentType::Text, false),
        ("nested.html.txt".into(), ContentType::Text, false),
        ("nested.html.j2.jinja".into(), ContentType::Text, false),
    ]);

    let engines = [
        #[cfg(feature = "tera")]
        ("tera", "&lt;&gt;&amp;&quot;&#39;/"),
        #[cfg(feature = "minijinja")]
        ("j2", "&lt;&gt;&amp;&quot;&#x27;&#x2f;"),
    ];

    let content = "<>&\"'/";
    for (engine, escaped) in engines {
        let dir = tempfile::tempdir().expect("template directory");
        for (name, _, _) in &cases {
            let path = dir.path().join(format!("{name}.{engine}"));
            std::fs::write(path, "{{ content }}").expect("write template");
        }

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(Templating::new());
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for (name, content_type, escape) in &cases {
            let expected = if *escape { escaped } else { content };
            assert_eq!(
                metadata.render(name.clone(), context! { content }),
                Some((content_type.clone(), expected.into())),
                "{engine}: {name}"
            );
        }

        assert!(!metadata.contains_template("same"));
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
        = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script /&gt;\n\nfoot";

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
        let template = Template::show(client.rocket(), "tera/txt_test.txt", &map);
        let md_rendered = metadata.render("tera/txt_test.txt", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::Text, UNESCAPED_EXPECTED.into())));

        // Now with an HTML file, which should escape.
        let template = Template::show(client.rocket(), "tera/html_test.html", &map);
        let md_rendered = metadata.render("tera/html_test.html", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::HTML, ESCAPED_EXPECTED.into())));
    }

    #[async_test]
    async fn test_tera_hooks() {
        use rocket::local::asynchronous::Client;
        use rocket_dyn_templates::tera::{Kwargs, State};

        let dir = tempfile::tempdir().expect("template directory");
        std::fs::write(
            dir.path().join("base.html.tera"),
            "{{ value | double }}:{% block content %}base{% endblock %}",
        ).expect("write template");

        let templating = Templating::new()
            .try_on_init(|_| Err("replaced initialization hook".into()))
            .on_load(|_| panic!("replaced loading hook"))
            .try_on_load(|engines| {
                engines.tera.add_raw_template("child.html", concat!(
                    "{% extends \"base.html\" %}",
                    "{% block content %}child{% endblock %}",
                ))?;

                Ok(())
            })
            .on_init(|engines| {
                engines.tera.register_filter("double", |x: i64, _: Kwargs, _: &State<'_>| x * 2);
            });

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        assert_eq!(
            metadata.render("base.html", context! { value: 21 }),
            Some((ContentType::HTML, "42:base".into()))
        );

        assert_eq!(
            metadata.render("child.html", context! { value: 21 }),
            Some((ContentType::HTML, "42:child".into()))
        );
    }

    #[async_test]
    async fn test_tera_escaping() {
        use rocket::local::asynchronous::Client;

        let dir = tempfile::tempdir().expect("template directory");
        let templating = Templating::new().try_on_load(|engines| {
            engines.tera.add_raw_templates([
                ("text.txt", "{{ content }}"),
                ("html.HTML", "{{ content }}"),
                ("include.HTML", r#"{% include "text.txt" %}"#),
                ("include.txt", r#"{% include "html.HTML" %}"#),
                ("capture.HTML", "{% set value %}{{ content }}{% endset %}{{ value }}"),
                ("safe.HTML", "{{ content | safe }}"),
                ("engine.html.tera", "{{ content }}"),
                ("engine.XML.tera", "{{ content }}"),
                ("engine.txt.tera", "{{ content }}"),
            ])?;

            Ok(())
        });

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for (name, content_type, expected) in [
            ("include.HTML", ContentType::HTML, "&lt;b&gt;"),
            ("include.txt", ContentType::Text, "<b>"),
            ("capture.HTML", ContentType::HTML, "&lt;b&gt;"),
            ("safe.HTML", ContentType::HTML, "<b>"),
            ("engine.html.tera", ContentType::Text, "&lt;b&gt;"),
            ("engine.XML.tera", ContentType::Text, "&lt;b&gt;"),
            ("engine.txt.tera", ContentType::Text, "<b>"),
        ] {
            assert_eq!(
                metadata.render(name, context! { content: "<b>" }),
                Some((content_type, expected.into())),
                "{name}"
            );
        }
    }

    #[async_test]
    async fn test_tera_escaping_override() {
        use rocket::local::asynchronous::Client;

        let dir = tempfile::tempdir().expect("template directory");
        for name in ["file.html.tera", "file.txt.tera"] {
            std::fs::write(dir.path().join(name), "{{ content }}").expect("write template");
        }

        let templating = Templating::init(|engines| engines.tera.autoescape_on([".txt"]))
            .try_on_load(|engines| {
                engines.tera.add_raw_template("added.txt", "{{ content }}")?;
                Ok(())
            });

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for (name, content_type, expected) in [
            ("file.html", ContentType::HTML, "<b>"),
            ("file.txt", ContentType::Text, "&lt;b&gt;"),
            ("added.txt", ContentType::Text, "&lt;b&gt;"),
        ] {
            assert_eq!(
                metadata.render(name, context! { content: "<b>" }),
                Some((content_type, expected.into())),
                "{name}"
            );
        }
    }

    #[async_test]
    async fn test_globby_paths() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        assert!(metadata.contains_template("tera/[test]/html_test.html"));
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
    //     let template =
    //         Template::show(client.rocket(), "tera/txt_test.txt", &map);
    //     assert_eq!(template, Some(EXPECTED.into()));
    // }

    #[test]
    fn test_template_metadata_with_tera() {
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/tera/txt_test.txt").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/tera/html_test.html").dispatch();
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
        let template = Template::show(client.rocket(), "hbs/test.html", &map);
        let md_rendered = metadata.render("hbs/test.html", &map);
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
    //     let template =
    //         Template::show(client.rocket(), "hbs/test.html", &map);
    //     assert_eq!(template, Some(EXPECTED.into()));
    // }

    #[test]
    fn test_template_metadata_with_handlebars() {
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/hbs/test.html").dispatch();
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
        let template = Template::show(client.rocket(), "j2/txt_test.txt", &map);
        let md_rendered = (&metadata).render("j2/txt_test.txt", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::Text, UNESCAPED_EXPECTED.into())));

        // Now with an HTML file, which should escaped.
        let template = Template::show(client.rocket(), "j2/html_test.html", &map);
        let md_rendered = metadata.render("j2/html_test.html", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        assert_eq!(md_rendered, Some((ContentType::HTML, ESCAPED_EXPECTED.into())));
    }

    #[async_test]
    async fn test_j2_escaping() {
        use rocket::local::asynchronous::Client;

        let dir = tempfile::tempdir().expect("template directory");
        let templating = Templating::new().try_on_load(|engines| {
            for (name, source) in [
                ("text.txt", "{{ content }}"),
                ("html.HTML", "{{ content }}"),
                ("include.HTML", r#"{% include "text.txt" %}"#),
                ("include.txt", r#"{% include "html.HTML" %}"#),
                ("capture.HTML", "{% set value %}{{ content }}{% endset %}{{ value }}"),
                ("safe.HTML", "{{ content | safe }}"),
            ] {
                engines.minijinja.add_template(name, source)?;
            }

            Ok(())
        });

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for (name, content_type, expected) in [
            ("include.HTML", ContentType::HTML, "<b>"),
            ("include.txt", ContentType::Text, "&lt;b&gt;"),
            ("capture.HTML", ContentType::HTML, "&lt;b&gt;"),
            ("safe.HTML", ContentType::HTML, "<b>"),
        ] {
            assert_eq!(
                metadata.render(name, context! { content: "<b>" }),
                Some((content_type, expected.into())),
                "{name}"
            );
        }
    }

    #[async_test]
    async fn test_j2_escaping_override() {
        use rocket::local::asynchronous::Client;
        use rocket_dyn_templates::minijinja::AutoEscape;

        let dir = tempfile::tempdir().expect("template directory");
        std::fs::write(dir.path().join("file.html.j2"), "{{ content }}")
            .expect("write template");

        let templating = Templating::init(|engines| {
            engines.minijinja.set_auto_escape_callback(|_| AutoEscape::None);
        }).try_on_load(|engines| {
            engines.minijinja.add_template("added.HTML", "{{ content }}")?;
            Ok(())
        });

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(templating);
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for name in ["file.html", "added.HTML"] {
            assert_eq!(
                metadata.render(name, context! { content: "<b>" }),
                Some((ContentType::HTML, "<b>".into())),
                "{name}"
            );
        }
    }

    #[async_test]
    async fn test_j2_json_escaping() {
        use rocket::local::asynchronous::Client;

        let dir = tempfile::tempdir().expect("template directory");
        let content = "</script>&'";
        let json = r#""</script>&'""#;
        let cases = [
            ("data.json", ContentType::JSON, json),
            ("data.json.jinja", ContentType::Text, json),
            ("upper.JSON", ContentType::JSON, content),
            ("upper.json.JINJA", ContentType::Text, content),
        ];

        for (name, _, _) in &cases {
            std::fs::write(dir.path().join(format!("{name}.j2")), "{{ content }}")
                .expect("write template");
        }

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(Templating::new());
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        for (name, content_type, expected) in cases {
            assert_eq!(
                metadata.render(name, context! { content }),
                Some((content_type, expected.into())),
                "{name}"
            );
        }

        let content = context! { content, values: [1, 2] };
        assert_eq!(
            metadata.render("data.json", context! { content }),
            Some((ContentType::JSON, r#"{"content":"</script>&'","values":[1,2]}"#.into()))
        );
    }

    #[async_test]
    async fn test_j2_tojson() {
        use rocket::local::asynchronous::Client;

        let dir = tempfile::tempdir().expect("template directory");
        std::fs::write(
            dir.path().join("script.html.j2"),
            "<script>const data = {{ data | tojson }};</script>",
        ).expect("write template");

        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(Templating::new());
        let client = Client::debug(rocket).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        let data = context! { content: "</script>&'", values: [1, 2] };

        assert_eq!(
            metadata.render("script.html", context! { data }),
            Some((ContentType::HTML, concat!(
                r#"<script>const data = {"content": "\u003c/script\u003e\u0026\u0027", "#,
                r#""values": [1, 2]};</script>"#,
            ).into()))
        );
    }

    #[async_test]
    async fn test_globby_paths() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        assert!(metadata.contains_template("j2/[test]/html_test.html"));
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

        let template = Template::show(client.rocket(), "j2/txt_test.txt", &map);
        assert_eq!(template, Some(EXPECTED.into()));
    }

    #[test]
    fn test_template_metadata_with_j2() {
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/j2/txt_test.txt").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/j2/html_test.html").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/j2/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let response = client.get("/hbs/txt_test").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}

#[async_test]
#[cfg(debug_assertions)]
async fn test_template_reload() {
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
    use std::time::{Duration, Instant};

    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use rocket::request::FromRequest;

    fn write_file(path: &Path, text: &str) {
        let mut file = File::create(path).expect("open file");
        file.write_all(text.as_bytes()).expect("write file");
        file.sync_all().expect("sync file");
    }

    async fn wait_for(client: &Client, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            // Dispatch any request to trigger a template reload.
            client.get("/").dispatch().await;
            if condition() {
                return;
            }

            assert!(Instant::now() < deadline, "failed to reload templates in 3s");
            rocket::tokio::time::sleep(Duration::from_millis(50)).await;
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
        let renamed = dir.path().join(format!("reload.html.{ext}"));

        // Set up the template before initializing Rocket so that it will be
        // picked up in the initial loading of templates.
        write_file(&path, "initial");
        let init_path = path.clone();
        let load_path = path.clone();
        let init_calls = Arc::new(AtomicUsize::new(0));
        let load_calls = Arc::new(AtomicUsize::new(0));
        let init_counter = init_calls.clone();
        let load_counter = load_calls.clone();
        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir.path())))
            .attach(Templating::try_init(move |_| {
                init_counter.fetch_add(1, SeqCst);
                if matches!(fs::read_to_string(&init_path), Ok(text) if text == "reject init") {
                    return Err("reject initialization".into());
                }

                Ok(())
            }).try_on_load(move |_| {
                load_counter.fetch_add(1, SeqCst);
                if matches!(fs::read_to_string(&load_path), Ok(text) if text == "reject load") {
                    return Err("reject loading".into());
                }

                Ok(())
            }))
            .mount("/", routes![is_reloading]);
        let client = Client::debug(rocket).await.unwrap();
        assert_eq!(init_calls.load(SeqCst), 1);
        assert_eq!(load_calls.load(SeqCst), 1);
        assert_eq!(client.get("/is_reloading").dispatch().await.status(), Status::Ok);

        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        let render = |name| metadata.render(name, context! { content: "<b>" });
        assert_eq!(render("reload.txt"), Some((ContentType::Text, "initial".into())));

        // Write a change and wait for the new content to be rendered.
        write_file(&path, "reload");
        wait_for(&client, || {
            render("reload.txt") == Some((ContentType::Text, "reload".into()))
        }).await;
        assert!(init_calls.load(SeqCst) > 1);
        assert_eq!(init_calls.load(SeqCst), load_calls.load(SeqCst));

        // An initialization error skips loading and preserves active templates.
        let initialized = init_calls.load(SeqCst);
        let loaded = load_calls.load(SeqCst);
        write_file(&path, "reject init");
        wait_for(&client, || init_calls.load(SeqCst) > initialized).await;
        assert_eq!(load_calls.load(SeqCst), loaded);
        assert_eq!(render("reload.txt"), Some((ContentType::Text, "reload".into())));

        // An error from the loading hook also preserves active templates.
        write_file(&path, "reject load");
        wait_for(&client, || load_calls.load(SeqCst) > loaded).await;
        assert_eq!(render("reload.txt"), Some((ContentType::Text, "reload".into())));

        let initialized = init_calls.load(SeqCst);
        let loaded = load_calls.load(SeqCst);
        write_file(&path, invalid);
        wait_for(&client, || init_calls.load(SeqCst) > initialized).await;
        if ext == "j2" {
            // MiniJinja parses lazily, so invalid edits fail to render.
            assert!(load_calls.load(SeqCst) > loaded);
            assert_eq!(render("reload.txt"), None);
        } else {
            // Native loading errors skip the hook and preserve active templates.
            assert_eq!(load_calls.load(SeqCst), loaded);
            assert_eq!(render("reload.txt"), Some((ContentType::Text, "reload".into())));
        }

        let repaired = if ext == "hbs" { "repaired &lt;b&gt;" } else { "repaired <b>" };
        let loaded = load_calls.load(SeqCst);
        write_file(&path, "repaired {{ content }}");
        wait_for(&client, || {
            load_calls.load(SeqCst) > loaded
                && render("reload.txt") == Some((ContentType::Text, repaired.into()))
        }).await;

        fs::rename(&path, &renamed).expect("rename template");
        wait_for(&client, || {
            render("reload.html") == Some((ContentType::HTML, "repaired &lt;b&gt;".into()))
        }).await;
        assert_eq!(render("reload.txt"), None);

        fs::remove_file(&renamed).expect("remove template");
        wait_for(&client, || render("reload.html").is_none()).await;

        // Dropping the client also stops the watcher before removing its root.
        drop(client);
        dir.close().expect("remove template directory");
    }
}
