#[macro_use] extern crate rocket;

#[get("/")]
fn index() { }

mod module {
    // This one has all the same macro inputs, and we need it to
    // generate a crate-wide unique identifier for the macro it
    // defines.
    #[get("/")]
    pub fn index() { }
}

// Makes sure that the hashing of the proc macro's call site span
// is enough, even if we're inside a declarative macro
macro_rules! gen_routes {
    () => {
        #[get("/")]
        pub fn index() { }

        pub mod two {
            #[get("/")]
            pub fn index() { }
        }
    }
}

mod module2 {
    gen_routes!();

    pub mod module3 {
        gen_routes!();
    }
}

#[test]
fn test_uri_reachability() {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    let rocket = rocket::build()
        .mount("/", routes![index])
        .mount("/module", routes![module::index])
        .mount("/module2", routes![module2::index])
        .mount("/module2/two", routes![module2::two::index])
        .mount("/module2/module3", routes![module2::module3::index])
        .mount("/module2/module3/two", routes![module2::module3::two::index]);

    let uris = rocket.routes()
        .map(|r| r.uri.base().to_string())
        .collect::<Vec<_>>();

    let client = Client::debug(rocket).unwrap();
    for uri in uris {
        let response = client.get(uri).dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}

#[test]
fn test_uri_calls() {
    let uris = [
        uri!(index()),
        uri!(module::index()),
        uri!(module2::index()),
        uri!(module2::two::index()),
        uri!(module2::module3::index()),
        uri!(module2::module3::two::index()),
    ];

    assert!(uris.iter().all(|uri| uri == "/"));
}
