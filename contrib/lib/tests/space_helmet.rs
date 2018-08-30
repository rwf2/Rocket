#![cfg_attr(test, feature(plugin, decl_macro))]
#![cfg_attr(test, plugin(rocket_codegen))]
#![feature(extern_prelude)]

extern crate rocket_contrib;
extern crate rocket;

#[cfg(feature = "space_helmet")]
mod space_helmet_tests {
    use rocket_contrib::space_helmet::*;
    use rocket::http::uri::Uri;
    use rocket::http::Status;
    use rocket::local::Client;

    #[get("/")]
    fn hello() -> &'static str {
        "Hello, world!"
    }

    macro_rules! check_header {
        ($response:ident, $header_name:expr, $header_param:expr) => {
            match $response.headers().get_one($header_name) {
                Some(str) => {
                    assert_eq!(str, $header_param);
                }
                None => {
                    panic!("missing header parameters");
                }
            }
        };
    }

    #[test]
    fn defaults_test() {
        let helmet = Helmet::new()
            .hsts(HSTSPolicy::default())
            .expect_ct(ExpectCTPolicy::default())
            .referrer_policy(ReferrerPolicy::default());
        let rocket = rocket::ignite().mount("/", routes![hello]).attach(helmet);
        let client = Client::new(rocket).unwrap();
        let mut response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some("Hello, world!".into()));

        check_header!(response, "X-XSS-Protection", "1; mode=block");
        check_header!(response, "X-Frame-Options", "SAMEORIGIN");
        check_header!(response, "X-Content-Type-Options", "nosniff");
        check_header!(
            response,
            "Strict-Transport-Security",
            format!("max-age={}", HSTS_MAX_AGE_DEFAULT)
        );
        check_header!(
            response,
            "Expect-CT",
            format!("max-age={}, enforce", EXPECTCT_MAX_AGE_DEFAULT)
        );
        check_header!(response, "Referrer-Policy", "no-referrer");
    }

    #[test]
    fn uri_test() {
        let allow_uri = Uri::parse("https://www.google.com").unwrap();
        let report_uri = Uri::parse("https://www.google.com").unwrap();
        let enforce_uri = Uri::parse("https://www.google.com").unwrap();
        let helmet = Helmet::new()
            .frameguard(FramePolicy::AllowFrom(allow_uri))
            .xss_protect(XSSPolicy::EnableReport(report_uri))
            .expect_ct(ExpectCTPolicy::ReportAndEnforce(30, enforce_uri));
        let rocket = rocket::ignite().mount("/", routes![hello]).attach(helmet);
        let client = Client::new(rocket).unwrap();
        let response = client.get("/").dispatch();
        check_header!(
            response,
            "X-Frame-Options",
            "ALLOW-FROM https://www.google.com"
        );
        check_header!(
            response,
            "X-XSS-Protection",
            "1; report=https://www.google.com"
        );
        check_header!(
            response,
            "Expect-CT",
            "max-age=30, enforce, report-uri=\"https://www.google.com\""
        );
    }
}
