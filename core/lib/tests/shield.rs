#[macro_use] extern crate rocket;

use rocket::Config;
use rocket::http::Status;
use rocket::local::blocking::{Client, LocalResponse};
use rocket::shield::*;

use time::Duration;

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
    ($shield:expr, $closure:expr) => {{
        let rocket = rocket::build().mount("/", routes![hello]).attach($shield);
        let client = Client::debug(rocket).unwrap();
        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        $closure(response)
    }}
}

#[test]
fn default_shield() {
    let client = Client::debug(rocket::build()).unwrap();
    let response = client.get("/").dispatch();
    assert_header!(response, "Permissions-Policy", "interest-cohort=()");
    assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
    assert_header!(response, "X-Content-Type-Options", "nosniff");

    let client = Client::debug(rocket::custom(Config::debug_default())).unwrap();
    let response = client.get("/").dispatch();
    assert_header!(response, "Permissions-Policy", "interest-cohort=()");
    assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
    assert_header!(response, "X-Content-Type-Options", "nosniff");
}

#[test]
fn shield_singleton() {
    let rocket = rocket::build().attach(Shield::new());
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/").dispatch();
    assert_no_header!(response, "Permissions-Policy");
    assert_no_header!(response, "X-Frame-Options");
    assert_no_header!(response, "X-Content-Type-Options");

    let rocket = rocket::custom(Config::debug_default()).attach(Shield::new());
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/").dispatch();
    assert_no_header!(response, "Permissions-Policy");
    assert_no_header!(response, "X-Frame-Options");
    assert_no_header!(response, "X-Content-Type-Options");
}

#[test]
fn default_headers_test() {
    dispatch!(Shield::default(), |response: LocalResponse<'_>| {
        assert_header!(response, "Permissions-Policy", "interest-cohort=()");
        assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
        assert_header!(response, "X-Content-Type-Options", "nosniff");
    })
}

#[test]
fn disable_headers_test() {
    let shield = Shield::default().disable::<Permission>();
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "X-Frame-Options", "SAMEORIGIN");
        assert_header!(response, "X-Content-Type-Options", "nosniff");
        assert_no_header!(response, "Permissions-Policy");
    });

    let shield = Shield::default().disable::<Frame>();
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "Permissions-Policy", "interest-cohort=()");
        assert_header!(response, "X-Content-Type-Options", "nosniff");
        assert_no_header!(response, "X-Frame-Options");
    });

    let shield = Shield::default()
        .disable::<Frame>()
        .disable::<Permission>()
        .disable::<NoSniff>();

    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_no_header!(response, "X-Frame-Options");
        assert_no_header!(response, "Permissions-Policy");
        assert_no_header!(response, "X-Content-Type-Options");
    });

    dispatch!(Shield::new(), |response: LocalResponse<'_>| {
        assert_no_header!(response, "X-Frame-Options");
        assert_no_header!(response, "Permissions-Policy");
        assert_no_header!(response, "X-Content-Type-Options");
    });
}

#[test]
fn additional_headers_test() {
    let shield = Shield::default()
        .enable(Hsts::default())
        .enable(ExpectCt::default())
        .enable(Referrer::default());

    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(
            response,
            "Strict-Transport-Security",
            format!("max-age={}", Duration::days(365).whole_seconds())
        );

        assert_header!(
            response,
            "Expect-CT",
            format!("max-age={}, enforce", Duration::days(30).whole_seconds())
        );

        assert_header!(response, "Referrer-Policy", "no-referrer");
    })
}

#[test]
fn uri_test() {
    let enforce_uri = uri!("https://rocket.rs");
    let shield = Shield::default()
        .enable(ExpectCt::ReportAndEnforce(Duration::seconds(30), enforce_uri));

    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "Expect-CT",
            "max-age=30, enforce, report-uri=\"https://rocket.rs\"");
    });
}

#[test]
fn prefetch_test() {
    let shield = Shield::default().enable(Prefetch::default());
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "X-DNS-Prefetch-Control", "off");
    });

    let shield = Shield::default().enable(Prefetch::Off);
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "X-DNS-Prefetch-Control", "off");
    });

    let shield = Shield::default().enable(Prefetch::On);
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "X-DNS-Prefetch-Control", "on");
    });
}

#[test]
#[should_panic]
fn bad_uri_permission_test() {
    let uri = uri!("http://:200");
    Permission::allowed(Feature::Usb, Allow::Origin(uri));
}

#[test]
#[should_panic]
fn bad_uri_permission_test2() {
    let uri = uri!("http://:200");
    Permission::default().allow(Feature::Camera, Allow::Origin(uri));
}

#[test]
fn permission_test() {
    let shield = Shield::default().enable(Permission::default());
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "Permissions-Policy", "interest-cohort=()");
    });

    let shield = Shield::default().enable(Permission::blocked(Feature::Usb));
    dispatch!(shield, |response: LocalResponse<'_>| {
        assert_header!(response, "Permissions-Policy", "usb=()");
    });

    let permission = Permission::blocked(Feature::Usb)
        .block(Feature::Camera)
        .block(Feature::WebShare);

    let shield = Shield::default().enable(permission);
    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy", "usb=(), camera=(), web-share=()");
    });

    let permission = Permission::blocked(Feature::Usb)
        .allow(Feature::Camera, [Allow::Any, Allow::This])
        .block(Feature::WebShare);

    let shield = Shield::default().enable(permission);
    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy", "usb=(), camera=(*), web-share=()");
    });

    let permission = Permission::blocked(Feature::Usb)
        .allow(Feature::Camera, [Allow::This])
        .block(Feature::WebShare);

    let shield = Shield::default().enable(permission);
    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy", "usb=(), camera=(self), web-share=()");
    });

    let uri = uri!("http://rocket.rs");
    let permission = Permission::allowed(Feature::Usb, Allow::Origin(uri))
        .allow(Feature::Camera, [Allow::This])
        .block(Feature::WebShare);

    let shield = Shield::default().enable(permission);
    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy",
            "usb=(\"http://rocket.rs\"), camera=(self), web-share=()");
    });

    let origin1 = Allow::Origin(uri!("http://rocket.rs"));
    let origin2 = Allow::Origin(uri!("https://rocket.rs"));
    let shield = Shield::default()
        .enable(Permission::allowed(Feature::Camera, [origin1, origin2]));

    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy",
            "camera=(\"http://rocket.rs\" \"https://rocket.rs\")");
    });

    let origin1 = Allow::Origin(uri!("http://rocket.rs"));
    let origin2 = Allow::Origin(uri!("https://rocket.rs"));
    let perm = Permission::allowed(Feature::Accelerometer, [origin1, origin2])
        .block(Feature::Usb);

    let shield = Shield::default().enable(perm);
    dispatch!(shield, |r: LocalResponse<'_>| {
        assert_header!(r, "Permissions-Policy",
            "accelerometer=(\"http://rocket.rs\" \"https://rocket.rs\"), usb=()");
    });
}
