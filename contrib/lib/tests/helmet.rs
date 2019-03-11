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
