#![feature(proc_macro_hygiene, decl_macro)]

#[cfg(feature = "templates")]
#[macro_use] extern crate rocket;

#[cfg(feature = "templates")]
mod templates_tests {
    use std::path::{Path, PathBuf};

    use rocket::{Rocket, http::RawStr};
    use rocket::config::{Config, Environment};
    use rocket_contrib::templates::{Template, Metadata};

    #[get("/<engine>/<name>")]
    fn template_check(md: Metadata<'_>, engine: &RawStr, name: &RawStr) -> Option<()> {
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

    fn rocket() -> Rocket {
        let config = Config::build(Environment::Development)
            .extra("template_dir", template_root().to_str().expect("template directory"))
            .expect("valid configuration");

        rocket::custom(config).attach(Template::fairing())
            .mount("/", routes![template_check, is_reloading])
    }

    #[cfg(feature = "tera_templates")]
    mod tera_tests {
        use super::*;
        use std::collections::HashMap;
        use rocket::http::Status;
        use rocket::local::Client;

        const UNESCAPED_EXPECTED: &'static str
            = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot\n";
        const ESCAPED_EXPECTED: &'static str
            = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script &#x2F;&gt;\n\nfoot\n";

        #[test]
        fn test_tera_templates() {
            let rocket = rocket();
            let mut map = HashMap::new();
            map.insert("title", "_test_");
            map.insert("content", "<script />");

            // Test with a txt file, which shouldn't escape.
            let template = Template::show(&rocket, "tera/txt_test", &map);
            assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));

            // Now with an HTML file, which should.
            let template = Template::show(&rocket, "tera/html_test", &map);
            assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        }

        #[test]
        fn test_template_metadata_with_tera() {
            let client = Client::new(rocket()).unwrap();

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

    #[cfg(feature = "handlebars_templates")]
    mod handlebars_tests {
        use super::*;
        use std::collections::HashMap;
        use rocket::http::Status;
        use rocket::local::Client;

        const EXPECTED: &'static str
            = "Hello _test_!\n\n<main> &lt;script /&gt; hi </main>\nDone.\n\n";

        #[test]
        fn test_handlebars_templates() {
            let rocket = rocket();
            let mut map = HashMap::new();
            map.insert("title", "_test_");
            map.insert("content", "<script /> hi");

            // Test with a txt file, which shouldn't escape.
            let template = Template::show(&rocket, "hbs/test", &map);
            assert_eq!(template, Some(EXPECTED.into()));
        }

        #[test]
        fn test_template_metadata_with_handlebars() {
            let client = Client::new(rocket()).unwrap();

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
            use std::thread;
            use std::time::Duration;

            use rocket::local::Client;

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
            let client = Client::new(rocket()).unwrap();
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
                thread::sleep(Duration::from_millis(250));
            }

            panic!("failed to reload modified template in 1.5s");
        }
    }
}
