use std::fs::{self, File};

use rocket::http::{CookieJar, Cookie};
use rocket::local::blocking::Client;
use rocket::fs::relative;

use crate::{DEFAULT_PROFILES, validate_profiles};

#[get("/cookie")]
fn cookie(jar: &CookieJar<'_>) {
    jar.add(("k1", "v1"));
    jar.add_private(("k2", "v2"));

    jar.add(Cookie::build(("k1u", "v1u")).secure(false));
    jar.add_private(Cookie::build(("k2u", "v2u")).secure(false));
}

#[test]
fn hello_mutual() {
    let client = Client::tracked_secure(crate::rocket()).unwrap();
    let cert_paths = fs::read_dir(relative!("private")).unwrap()
        .map(|entry| entry.unwrap().path().to_string_lossy().into_owned())
        .filter(|path| path.ends_with("_cert.pem") && !path.ends_with("ca_cert.pem"));

    for path in cert_paths {
        let response = client.get("/")
            .identity(File::open(&path).unwrap())
            .dispatch();

        let response = response.into_string().unwrap();
        let subject = response.split(']').nth(1).unwrap().trim();
        assert_eq!(subject, "C=US, ST=CA, O=Rocket, CN=localhost");
    }
}

#[test]
fn secure_cookies() {
    let rocket = crate::rocket().mount("/", routes![cookie]);
    let client = Client::tracked_secure(rocket).unwrap();

    let response = client.get("/cookie").dispatch();
    let c1 = response.cookies().get("k1").unwrap();
    let c2 = response.cookies().get_private("k2").unwrap();
    let c3 = response.cookies().get("k1u").unwrap();
    let c4 = response.cookies().get_private("k2u").unwrap();

    assert_eq!(c1.secure(), Some(true));
    assert_eq!(c2.secure(), Some(true));
    assert_ne!(c3.secure(), Some(true));
    assert_ne!(c4.secure(), Some(true));
}

#[test]
fn insecure_cookies() {
    let rocket = crate::rocket().mount("/", routes![cookie]);
    let client = Client::tracked(rocket).unwrap();

    let response = client.get("/cookie").dispatch();
    let c1 = response.cookies().get("k1").unwrap();
    let c2 = response.cookies().get_private("k2").unwrap();
    let c3 = response.cookies().get("k1u").unwrap();
    let c4 = response.cookies().get_private("k2u").unwrap();

    assert_eq!(c1.secure(), None);
    assert_eq!(c2.secure(), None);
    assert_eq!(c3.secure(), None);
    assert_eq!(c4.secure(), None);
}

#[test]
fn validate_tls_profiles() {
    validate_profiles(DEFAULT_PROFILES);
}
