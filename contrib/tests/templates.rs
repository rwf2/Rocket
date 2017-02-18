extern crate rocket;
extern crate rocket_contrib;

use std::env;
use rocket::config::Config;
use rocket::config::Environment::*;

fn init() {
    let cwd = env::current_dir().expect("current working directory");
    let tests_dir = cwd.join("tests");

    let config = Config::build(Development).root(tests_dir).unwrap();
    rocket::custom(config, true);
}

// FIXME: Do something about overlapping configs.
#[cfg(feature = "tera_templates")]
mod tera_tests {
    use super::*;
    use rocket_contrib::Template;
    use std::collections::HashMap;

    const UNESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot\n";
    const ESCAPED_EXPECTED: &'static str
        = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script &#x2F;&gt;\n\nfoot\n";

    #[test]
    fn test_tera_templates() {
        init();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");

        // Test with a txt file, which shouldn't escape.
        let template = Template::render("tera/txt_test", &map);
        assert_eq!(&template.to_string(), UNESCAPED_EXPECTED);

        // Now with an HTML file, which should.
        let template = Template::render("tera/html_test", &map);
        assert_eq!(&template.to_string(), ESCAPED_EXPECTED);
    }
}

#[cfg(feature = "handlebars_templates")]
mod handlebars_tests {
    use super::*;
    use rocket_contrib::Template;
    use std::collections::HashMap;

    const EXPECTED: &'static str
        = "Hello _test_!\n\n<main> &lt;script /&gt; hi </main>\nDone.\n\n";

    #[test]
    fn test_handlebars_templates() {
        init();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script /> hi");

        // Test with a txt file, which shouldn't escape.
        let template = Template::render("hbs/test", &map);
        assert_eq!(&template.to_string(), EXPECTED);
    }
}

