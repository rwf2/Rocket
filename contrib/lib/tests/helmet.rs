#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
#[cfg(feature = "helmet")]
extern crate rocket;

#[cfg(feature = "helmet")]
mod helmet_tests {
    extern crate time;
    extern crate rocket_contrib;

    use rocket;
    use rocket::http::{Status, uri::Uri};
    use rocket::local::{Client, LocalResponse};

    use self::rocket_contrib::helmet::*;
    use self::time::Duration;

    #[get("/")] fn hello() { }

    macro_rules! assert_header {
        ($response:ident, $name:expr, $value:expr) => {
            match $response.headers().get_one($name) {
                Some(value) => assert_eq!(value, $value),
                None => panic!("missing header '{}' with value '{}'", $name, $value)
            }
        };
    }

    macro_rules! assert_no_header {
        ($response:ident, $name:expr) => {
            if let Some(value) = $response.headers().get_one($name) {
                panic!("unexpected header: '{}={}", $name, value);
            }
        };
    }

    macro_rules! dispatch {
        ($helmet:expr, $closure:expr) => {{
            let rocket = rocket::ignite().mount("/", routes![hello]).attach($helmet);
            let client = Client::new(rocket).unwrap();
            let response = client.get("/").dispatch();
            assert_eq!(response.status(), Status::Ok);
            $closure(response)
        }}
    }

    #[test]
    fn default_headers_test() {
        dispatch!(SpaceHelmet::default(), |response: LocalResponse| {
            assert_header!(response, "X-XSS-Protection", "1");
            assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
            assert_header!(response, "X-Content-Type-Options", "nosniff");
        })
    }

    #[test]
    fn disable_headers_test() {
        let helmet = SpaceHelmet::default().disable::<XssFilter>();
        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
            assert_header!(response, "X-Content-Type-Options", "nosniff");
            assert_no_header!(response, "X-XSS-Protection");
        });

        let helmet = SpaceHelmet::default().disable::<Frame>();
        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(response, "X-XSS-Protection", "1");
            assert_header!(response, "X-Content-Type-Options", "nosniff");
            assert_no_header!(response, "X-Frame-Options");
        });

        let helmet = SpaceHelmet::default()
            .disable::<Frame>()
            .disable::<XssFilter>()
            .disable::<NoSniff>();

        dispatch!(helmet, |response: LocalResponse| {
            assert_no_header!(response, "X-Frame-Options");
            assert_no_header!(response, "X-XSS-Protection");
            assert_no_header!(response, "X-Content-Type-Options");
        });

        dispatch!(SpaceHelmet::new(), |response: LocalResponse| {
            assert_no_header!(response, "X-Frame-Options");
            assert_no_header!(response, "X-XSS-Protection");
            assert_no_header!(response, "X-Content-Type-Options");
        });
    }

    #[test]
    fn additional_headers_test() {
        let helmet = SpaceHelmet::default()
            .enable(Hsts::default())
            .enable(ExpectCt::default())
            .enable(Referrer::default());

        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(
                response,
                "Strict-Transport-Security",
                format!("max-age={}", Duration::weeks(52).num_seconds())
            );

            assert_header!(
                response,
                "Expect-CT",
                format!("max-age={}, enforce", Duration::days(30).num_seconds())
            );

            assert_header!(response, "Referrer-Policy", "no-referrer");
        })
    }

    #[test]
    fn content_security_policy_test() {
        let default_src = "https://default.rocket.rs".to_string();
        let script_src = "https://script.rocket.rs".to_string();
        let style_src = "https://style.rocket.rs".to_string();
        let img_src = "https://img.rocket.rs".to_string();
        let connect_src = "https://connect.rocket.rs".to_string();
        let font_src = "https://font.rocket.rs".to_string();
        let object_src = "https://object.rocket.rs".to_string();
        let media_src = "https://media.rocket.rs".to_string();
        let child_src = "https://child.rocket.rs".to_string();
        let report_uri = "https://report.rocket.rs".to_string();

        let defaults = Directive::DefaultSrc(vec![
            Box::new(KeywordSource::Noney),
            Box::new(HostSource::Uri(default_src)),
        ]);

        let scripts = Directive::ScriptSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(KeywordSource::UnsafeInline),
            Box::new(KeywordSource::UnsafeEval),
            Box::new(NonceSouce::Value("f00b4r".to_string())),
            Box::new(HashSource::Sha256("qznLcsROx4GACP2dm0UCKCzCG+HiZ1guq6ZZDob/Tng=".to_string())),
            Box::new(HashSource::Sha384("qzn...384=".to_string())),
            Box::new(HashSource::Sha512("qzn...512=".to_string())),
            Box::new(HostSource::Uri(script_src)),
        ]);

        let styles = Directive::StyleSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(KeywordSource::UnsafeInline),
            Box::new(HostSource::Uri(style_src)),
        ]);

        let images = Directive::ImgSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(KeywordSource::Data),
            Box::new(KeywordSource::Blob),
            Box::new(SchemeSource::Https),
            Box::new(HostSource::Uri(img_src)),
        ]);

        let connects = Directive::ConnectSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(HostSource::Uri(connect_src)),
        ]);

        let fonts = Directive::FontSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(HostSource::Uri(font_src)),
        ]);

        let objects = Directive::ObjectSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(HostSource::Uri(object_src)),
        ]);

        let medias = Directive::MediaSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(HostSource::Uri(media_src)),
        ]);

        let childs = Directive::ChildSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(HostSource::Uri(child_src)),
        ]);

        let report = Directive::ReportUri(
            HostSource::Uri(report_uri)
        );

        let directives = vec![
            defaults,
            scripts,
            styles,
            images,
            connects,
            fonts,
            objects,
            medias,
            childs,
            report
        ];

        let csp = ContentSecurityPolicy::Enable(directives);
        let helmet = SpaceHelmet::default().enable(csp);

        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(
                response,
                "Content-Security-Policy",
                "default-src: \'none\' https://default.rocket.rs; script-src: \'self\' \'unsafe-inline\' \'unsafe-eval\' 'nonce-f00b4r' 'sha256-qznLcsROx4GACP2dm0UCKCzCG+HiZ1guq6ZZDob/Tng=' 'sha384-qzn...384=' 'sha512-qzn...512=' https://script.rocket.rs; style-src: \'self\' \'unsafe-inline\' https://style.rocket.rs; img-src: \'self\' data: blob: https: https://img.rocket.rs; connect-src: \'self\' https://connect.rocket.rs; font-src: \'self\' https://font.rocket.rs; object-src: \'self\' https://object.rocket.rs; media-src: \'self\' https://media.rocket.rs; child-src: \'self\' https://child.rocket.rs; report-uri: https://report.rocket.rs"
            );
        })
    }

    #[test]
    fn content_security_policy_report_only_test() {
        let scripts = Directive::ScriptSrc(vec![
            Box::new(KeywordSource::Selfy),
            Box::new(NonceSouce::Value("f00b4r".to_string())),
        ]);

        let directives = vec![scripts];
        let csp = ContentSecurityPolicy::ReportOnly(directives);
        let helmet = SpaceHelmet::default().enable(csp);

        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(
                response,
                "Content-Security-Policy-Report-Only",
                "script-src: \'self\' 'nonce-f00b4r'"
            );
        })
    }
    #[test]
    fn uri_test() {
        let allow_uri = Uri::parse("https://www.google.com").unwrap();
        let report_uri = Uri::parse("https://www.google.com").unwrap();
        let enforce_uri = Uri::parse("https://www.google.com").unwrap();

        let helmet = SpaceHelmet::default()
            .enable(Frame::AllowFrom(allow_uri))
            .enable(XssFilter::EnableReport(report_uri))
            .enable(ExpectCt::ReportAndEnforce(Duration::seconds(30), enforce_uri));

        dispatch!(helmet, |response: LocalResponse| {
            assert_header!(response, "X-Frame-Options",
                           "ALLOW-FROM https://www.google.com");

            assert_header!(response, "X-XSS-Protection",
                           "1; report=https://www.google.com");

            assert_header!(response, "Expect-CT",
                "max-age=30, enforce, report-uri=\"https://www.google.com\"");
        });
    }
}
