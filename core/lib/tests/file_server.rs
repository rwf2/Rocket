use std::{io::Read, fs};
use std::path::Path;

use rocket::{Rocket, Route, Build};
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::fs::{FileServer, relative, rewrite::*};

fn static_root() -> &'static Path {
    Path::new(relative!("/tests/static"))
}

fn rocket() -> Rocket<Build> {
    let root = static_root();
    rocket::build()
        .mount("/default", FileServer::new(&root))
        .mount(
            "/no_index",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(Prefix::checked(&root))
        )
        .mount(
            "/dots",
            FileServer::identity()
                .rewrite(Prefix::checked(&root))
        )
        .mount(
            "/index",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(Prefix::checked(&root))
                .rewrite(DirIndex::unconditional("index.html"))
        )
        .mount(
            "/try_index",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(Prefix::checked(&root))
                .rewrite(DirIndex::if_exists("index.html"))
                .rewrite(DirIndex::if_exists("index.htm"))
        )
        .mount(
            "/both",
            FileServer::identity()
                .rewrite(Prefix::checked(&root))
                .rewrite(DirIndex::unconditional("index.html"))
        )
        .mount(
            "/redir",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(Prefix::checked(&root))
                .rewrite(TrailingDirs)
        )
        .mount(
            "/redir_index",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(Prefix::checked(&root))
                .rewrite(TrailingDirs)
                .rewrite(DirIndex::unconditional("index.html"))
        )
        .mount(
            "/index_file",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(File::checked(root.join("other/hello.txt")))
        )
        .mount(
            "/missing_root",
            FileServer::identity()
                .filter(|f, _| f.is_visible())
                .rewrite(File::new(root.join("no_file")))
        )
}

static REGULAR_FILES: &[&str] = &[
    "index.html",
    "inner/goodbye",
    "inner/index.html",
    "other/hello.txt",
    "other/index.htm",
];

static HIDDEN_FILES: &[&str] = &[
    ".hidden",
    "inner/.hideme",
];

static INDEXED_DIRECTORIES: &[&str] = &[
    "",
    "inner/",
];

fn assert_file_matches(client: &Client, prefix: &str, path: &str, disk_path: Option<&str>) {
    let full_path = format!("/{}/{}", prefix, path);
    let response = client.get(full_path).dispatch();
    if let Some(disk_path) = disk_path {
        assert_eq!(response.status(), Status::Ok);

        let mut path = static_root().join(disk_path);
        if path.is_dir() {
            path = path.join("index.html");
        }

        let mut file = fs::File::open(path).expect("open file");
        let mut expected_contents = String::new();
        file.read_to_string(&mut expected_contents).expect("read file");
        assert_eq!(response.into_string(), Some(expected_contents));
    } else {
        assert_eq!(response.status(), Status::NotFound);
    }
}

fn assert_file(client: &Client, prefix: &str, path: &str, exists: bool) {
    if exists {
        assert_file_matches(client, prefix, path, Some(path))
    } else {
        assert_file_matches(client, prefix, path, None)
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
fn test_alt_roots() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_file(&client, "missing_root", "", false);
    assert_file_matches(&client, "index_file", "", Some("other/hello.txt"));
}

#[test]
fn test_allow_special_dotpaths() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_file_matches(&client, "no_index", "./index.html", Some("index.html"));
    assert_file_matches(&client, "no_index", "foo/../index.html", Some("index.html"));
    assert_file_matches(&client, "no_index", "inner/./index.html", Some("inner/index.html"));
    assert_file_matches(&client, "no_index", "../index.html", Some("index.html"));
}

#[test]
fn test_try_index() {
    let client = Client::debug(rocket()).expect("valid rocket");
    assert_file_matches(&client, "try_index", "inner", Some("inner/index.html"));
    assert_file_matches(&client, "try_index", "other", Some("other/index.htm"));
}

#[test]
fn test_ranking() {
    let root = static_root();
    for rank in -128..128 {
        let a = FileServer::new(&root).rank(rank);
        let b = FileServer::new(&root).rank(rank);

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

    // Redirection only happens if enabled, and doesn't affect index behavior.
    let response = client.get("/no_index/inner").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/index/inner").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/redir/inner").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/"));

    let response = client.get("/redir/inner?foo=bar").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/?foo=bar"));

    let response = client.get("/redir_index/inner").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir_index/inner/"));

    // Paths with trailing slash are unaffected.
    let response = client.get("/redir/inner/").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/redir_index/inner/").dispatch();
    assert_eq!(response.status(), Status::Ok);

    // Root of route is also redirected.
    let response = client.get("/no_index/").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/index/").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/redir/inner").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/"));

    let response = client.get("/redir/other").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/other/"));

    let response = client.get("/redir_index/other").dispatch();
    assert_eq!(response.status(), Status::TemporaryRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir_index/other/"));
}

#[test]
#[should_panic]
fn test_panic_on_missing_file() {
    let _ = File::checked(static_root().join("missing_file"));
}

#[test]
#[should_panic]
fn test_panic_on_missing_dir() {
    let _ = Prefix::checked(static_root().join("missing_dir"));
}

#[test]
#[should_panic]
fn test_panic_on_file_not_dir() {
    let _ = Prefix::checked(static_root().join("index.html"));
}
