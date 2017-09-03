extern crate rocket_contrib;

use std::env;
use std::path::PathBuf;

fn template_root() -> PathBuf {
    let cwd = env::current_dir().expect("current working directory");
    cwd.join("tests").join("templates")
}

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
        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(template_root(), "tera/txt_test", &map);
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));

        // Now with an HTML file, which should.
        let template = Template::show(template_root(), "tera/html_test", &map);
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
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
        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script /> hi");

        // Test with a txt file, which shouldn't escape.
        let template = Template::show(template_root(), "hbs/test", &map);
        assert_eq!(template, Some(EXPECTED.into()));
    }
}

