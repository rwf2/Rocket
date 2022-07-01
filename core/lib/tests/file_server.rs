use std::{io::Read, fs::File};
use std::path::Path;

use rocket::{Rocket, Route, Build};
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::fs::{FileServer, Options, relative};

fn static_root() -> &'static Path {
    Path::new(relative!("/tests/static"))
}

fn rocket() -> Rocket<Build> {
    let root = static_root();
    rocket::build()
        .mount("/default", FileServer::from(&root))
        .mount("/no_index", FileServer::new(&root, Options::None))
        .mount("/dots", FileServer::new(&root, Options::DotFiles))
        .mount("/index", FileServer::new(&root, Options::Index))
        .mount("/both", FileServer::new(&root, Options::DotFiles | Options::Index))
        .mount("/redir", FileServer::new(&root, Options::NormalizeDirs))
        .mount("/redir_index", FileServer::new(&root, Options::NormalizeDirs | Options::Index))
}

static REGULAR_FILES: &[&str] = &[
    "index.html",
    "inner/goodbye",
    "inner/index.html",
    "other/hello.txt",
];

static HIDDEN_FILES: &[&str] = &[
    ".hidden",
    "inner/.hideme",
];

static INDEXED_DIRECTORIES: &[&str] = &[
    "",
    "inner/",
];

fn assert_file(client: &Client, prefix: &str, path: &str, exists: bool) {
    let full_path = format!("/{}/{}", prefix, path);
    let response = client.get(full_path).dispatch();
    if exists {
        assert_eq!(response.status(), Status::Ok);

        let mut path = static_root().join(path);
        if path.is_dir() {
            path = path.join("index.html");
        }

        let mut file = File::open(path).expect("open file");
        let mut expected_contents = String::new();
        file.read_to_string(&mut expected_contents).expect("read file");
        assert_eq!(response.into_string(), Some(expected_contents));
    } else {
        assert_eq!(response.status(), Status::NotFound);
    }
}

fn assert_all(client: &Client, prefix: &str, paths: &[&str], exist: bool) {
    for path in paths.iter() {
        assert_file(client, prefix, path, exist);
    }
}

#[test]
fn test_static_no_index() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_all(&client, "no_index", REGULAR_FILES, true);
    assert_all(&client, "no_index", HIDDEN_FILES, false);
    assert_all(&client, "no_index", INDEXED_DIRECTORIES, false);
}

#[test]
fn test_static_hidden() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_all(&client, "dots", REGULAR_FILES, true);
    assert_all(&client, "dots", HIDDEN_FILES, true);
    assert_all(&client, "dots", INDEXED_DIRECTORIES, false);
}

#[test]
fn test_static_index() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_all(&client, "index", REGULAR_FILES, true);
    assert_all(&client, "index", HIDDEN_FILES, false);
    assert_all(&client, "index", INDEXED_DIRECTORIES, true);

    assert_all(&client, "default", REGULAR_FILES, true);
    assert_all(&client, "default", HIDDEN_FILES, false);
    assert_all(&client, "default", INDEXED_DIRECTORIES, true);
}

#[test]
fn test_static_all() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_all(&client, "both", REGULAR_FILES, true);
    assert_all(&client, "both", HIDDEN_FILES, true);
    assert_all(&client, "both", INDEXED_DIRECTORIES, true);
}

#[test]
fn test_ranking() {
    let root = static_root();
    for rank in -128..128 {
        let a = FileServer::new(&root, Options::None).rank(rank);
        let b = FileServer::from(&root).rank(rank);

        for handler in vec![a, b] {
            let routes: Vec<Route> = handler.into();
            assert!(routes.iter().all(|route| route.rank == rank), "{}", rank);
        }
    }
}

#[test]
fn test_forwarding() {
    use rocket::{get, routes};

    #[get("/<value>", rank = 20)]
    fn catch_one(value: String) -> String { value }

    #[get("/<a>/<b>", rank = 20)]
    fn catch_two(a: &str, b: &str) -> String { format!("{}/{}", a, b) }

    let rocket = rocket().mount("/default", routes![catch_one, catch_two]);
    let client = Client::debug(rocket).expect("valid rocket");

    let response = client.get("/default/ireallydontexist").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "ireallydontexist");

    let response = client.get("/default/idont/exist").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "idont/exist");

    assert_all(&client, "both", REGULAR_FILES, true);
    assert_all(&client, "both", HIDDEN_FILES, true);
    assert_all(&client, "both", INDEXED_DIRECTORIES, true);
}

#[test]
fn test_redirection() {
    let client = Client::debug(rocket()).expect("valid rocket");

    // Redirection only happens if enabled, and doesn't affect index behaviour.
    let response = client.get("/no_index/inner").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/index/inner").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/redir/inner").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/"));

    let response = client.get("/redir/inner?foo=bar").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/?foo=bar"));

    let response = client.get("/redir_index/inner").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir_index/inner/"));

    // Paths with trailing slash are unaffected.
    let response = client.get("/redir/inner/").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/redir_index/inner/").dispatch();
    assert_eq!(response.status(), Status::Ok);

    // Root of route is also redirected.
    let response = client.get("/no_index").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/index").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/redir").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/"));

    let response = client.get("/redir_index").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir_index/"));
}
