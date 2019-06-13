#![feature(proc_macro_hygiene, decl_macro)]

#[cfg(feature = "serve")]
mod static_tests {
    use std::{io::Read, fs::File};
    use std::path::{Path, PathBuf};

    use rocket::{self, Rocket, Route};
    use rocket_contrib::serve::{StaticFiles, Options};
    use rocket::http::Status;
    use rocket::local::Client;

    fn static_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("static")
    }

    fn rocket() -> Rocket {
        let root = static_root();
        rocket::ignite()
            .mount("/default", StaticFiles::from(&root))
            .mount("/no_index", StaticFiles::new(&root, Options::None))
            .mount("/dots", StaticFiles::new(&root, Options::DotFiles))
            .mount("/index", StaticFiles::new(&root, Options::Index))
            .mount("/both", StaticFiles::new(&root, Options::DotFiles | Options::Index))
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
        let full_path = format!("/{}", Path::new(prefix).join(path).display());
        let mut response = client.get(full_path).dispatch();
        if exists {
            assert_eq!(response.status(), Status::Ok);

            let mut path = static_root().join(path);
            if path.is_dir() {
                path = path.join("index.html");
            }

            let mut file = File::open(path).expect("open file");
            let mut expected_contents = String::new();
            file.read_to_string(&mut expected_contents).expect("read file");
            assert_eq!(response.body_string(), Some(expected_contents));
        } else {
            assert_eq!(response.status(), Status::NotFound);
        }
    }

    fn assert_all(client: &Client, prefix: &str, paths: &[&str], exist: bool) {
        paths.iter().for_each(|path| assert_file(client, prefix, path, exist))
    }

    #[test]
    fn test_static_no_index() {
        let client = Client::new(rocket()).expect("valid rocket");
        assert_all(&client, "no_index", REGULAR_FILES, true);
        assert_all(&client, "no_index", HIDDEN_FILES, false);
        assert_all(&client, "no_index", INDEXED_DIRECTORIES, false);
    }

    #[test]
    fn test_static_hidden() {
        let client = Client::new(rocket()).expect("valid rocket");
        assert_all(&client, "dots", REGULAR_FILES, true);
        assert_all(&client, "dots", HIDDEN_FILES, true);
        assert_all(&client, "dots", INDEXED_DIRECTORIES, false);
    }

    #[test]
    fn test_static_index() {
        let client = Client::new(rocket()).expect("valid rocket");
        assert_all(&client, "index", REGULAR_FILES, true);
        assert_all(&client, "index", HIDDEN_FILES, false);
        assert_all(&client, "index", INDEXED_DIRECTORIES, true);

        assert_all(&client, "default", REGULAR_FILES, true);
        assert_all(&client, "default", HIDDEN_FILES, false);
        assert_all(&client, "default", INDEXED_DIRECTORIES, true);
    }

    #[test]
    fn test_static_all() {
        let client = Client::new(rocket()).expect("valid rocket");
        assert_all(&client, "both", REGULAR_FILES, true);
        assert_all(&client, "both", HIDDEN_FILES, true);
        assert_all(&client, "both", INDEXED_DIRECTORIES, true);
    }

    #[test]
    fn test_ranking() {
        let root = static_root();
        for rank in -128..128 {
            let a = StaticFiles::new(&root, Options::None).rank(rank);
            let b = StaticFiles::from(&root).rank(rank);

            for handler in vec![a, b] {
                let routes: Vec<Route> = handler.into();
                assert!(routes.iter().all(|route| route.rank == rank), "{}", rank);
            }
        }
    }
}
