use std::{io::Read, fs::File};
use std::path::Path;

use rocket::{Rocket, Route, Build};
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::fs::{
    dir_root,
    file_root,
    filter_dotfiles,
    index,
    file_root_permissive,
    normalize_dirs,
    relative,
    FileServer
};

fn static_root() -> &'static Path {
    Path::new(relative!("/tests/static"))
}

fn rocket() -> Rocket<Build> {
    let root = static_root();
    rocket::build()
        .mount("/default", FileServer::from(&root))
        .mount(
            "/no_index",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(dir_root(&root))
        )
        .mount(
            "/dots",
            FileServer::empty()
                .map_file(dir_root(&root))
        )
        .mount(
            "/index",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(dir_root(&root))
                .map_file(index("index.html"))
        )
        .mount(
            "/both",
            FileServer::empty()
                .map_file(dir_root(&root))
                .map_file(index("index.html"))
        )
        .mount(
            "/redir",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(dir_root(&root))
                .map_file(normalize_dirs)
        )
        .mount(
            "/redir_index",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(dir_root(&root))
                .map_file(normalize_dirs)
                .map_file(index("index.html"))
        )
        .mount(
            "/index_file",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(file_root(root.join("other/hello.txt")))
        )
        .mount(
            "/missing_root",
            FileServer::empty()
                .filter_file(filter_dotfiles)
                .map_file(file_root_permissive(root.join("no_file")))
        )
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

fn assert_file_matches(client: &Client, prefix: &str, path: &str, disk_path: Option<&str>) {
    let full_path = format!("/{}/{}", prefix, path);
    let response = client.get(full_path).dispatch();
    if let Some(disk_path) = disk_path {
        assert_eq!(response.status(), Status::Ok);

        let mut path = static_root().join(disk_path);
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
fn test_ranking() {
    let root = static_root();
    for rank in -128..128 {
        let a = FileServer::new(&root, rank);
        let b = FileServer::new(&root, rank);

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
    let response = client.get("/no_index/").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/index/").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/redir/inner").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/inner/"));

    let response = client.get("/redir/other").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir/other/"));

    let response = client.get("/redir_index/other").dispatch();
    assert_eq!(response.status(), Status::PermanentRedirect);
    assert_eq!(response.headers().get("Location").next(), Some("/redir_index/other/"));
}

#[test]
#[should_panic]
fn test_panic_on_missing_file() {
    let _ = file_root(static_root().join("missing_file"));
}

#[test]
#[should_panic]
fn test_panic_on_missing_dir() {
    let _ = dir_root(static_root().join("missing_dir"));
}

#[test]
#[should_panic]
fn test_panic_on_file_not_dir() {
    let _ = dir_root(static_root().join("index.html"));
}
