#[macro_use] extern crate rocket;

use rocket::data::{Capped, Limits, ToByteUnit};
use rocket::form::{Form, FromFormField, ValueField, DataField};
use rocket::fs::TempFile;
use rocket::http::{ContentType, Header, Status};
use rocket::tokio::io::AsyncReadExt;
use rocket::local::blocking::Client;

#[post("/", data = "<form>")]
fn form(form: Form<&[u8]>) -> Vec<u8> {
    form.to_vec()
}

#[post("/number", data = "<number>")]
fn number(number: Form<usize>) -> String {
    number.to_string()
}

fn multipart(disposition: &str, value: &[u8]) -> Vec<u8> {
    let mut body = format!("--X\r\nContent-Disposition: form-data; {disposition}\r\n\r\n")
        .into_bytes();

    body.extend(value);
    body.extend(b"\r\n--X--\r\n");
    body
}

#[test]
fn test_multipart_number_without_content_type() {
    let client = Client::debug_with(routes![number]).unwrap();
    // A leading BOM is preserved, so it makes an otherwise valid number invalid.
    for (value, status) in [
        (&b"42"[..], Status::Ok),
        (&b"\xef\xbb\xbf42"[..], Status::UnprocessableEntity),
        (&b"\xff\xfe4\x002\x00"[..], Status::UnprocessableEntity),
        (&b"\xfe\xff\x004\x002"[..], Status::UnprocessableEntity),
    ] {
        let response = client.post("/number")
            .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
            .body(multipart(r#"name="number""#, value))
            .dispatch();

        assert_eq!(response.status(), status);
        if status == Status::Ok {
            assert_eq!(response.into_string().unwrap(), "42");
        }
    }
}

#[derive(Responder)]
struct FileContents {
    bytes: Vec<u8>,
    name: Header<'static>,
    content_type: Header<'static>,
    on_disk: Header<'static>,
    complete: Header<'static>,
}

async fn read_file(file: &TempFile<'_>, complete: bool) -> FileContents {
    let mut bytes = Vec::new();
    file.open().await.unwrap().read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes.len() as u64, file.len());
    let name = file.raw_name()
        .map(|n| n.dangerous_unsafe_unsanitized_raw().as_str());
    FileContents {
        bytes,
        name: Header::new("X-File-Name", name.unwrap_or("<none>").to_string()),
        content_type: Header::new("X-Field-Type", file.content_type()
            .map(|c| c.to_string()).unwrap_or_default()),
        on_disk: Header::new("X-On-Disk", file.path().is_some().to_string()),
        complete: Header::new("X-Complete", complete.to_string()),
    }
}

#[post("/contents", data = "<file>")]
async fn contents(file: Form<TempFile<'_>>) -> FileContents {
    read_file(&file, true).await
}

#[post("/capped-file", data = "<file>")]
async fn capped_file(file: Form<Capped<TempFile<'_>>>) -> FileContents {
    read_file(&file, file.is_complete()).await
}

#[post("/string", data = "<value>")]
fn string(value: Form<String>) -> String {
    value.into_inner()
}

#[derive(Responder)]
struct CappedContents {
    bytes: Vec<u8>,
    complete: Header<'static>,
}

#[post("/capped-bytes", data = "<value>")]
fn capped_bytes(value: Form<Capped<&[u8]>>) -> CappedContents {
    CappedContents {
        bytes: value.value.to_vec(),
        complete: Header::new("X-Complete", value.is_complete().to_string()),
    }
}

#[post("/capped-string", data = "<value>")]
fn capped_string(value: Form<Capped<String>>) -> CappedContents {
    CappedContents {
        bytes: value.value.as_bytes().to_vec(),
        complete: Header::new("X-Complete", value.is_complete().to_string()),
    }
}

struct Dispatch(&'static str);

#[rocket::async_trait]
impl<'v> FromFormField<'v> for Dispatch {
    fn from_value(_: ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Dispatch("value"))
    }

    async fn from_data(_: DataField<'v, '_>) -> rocket::form::Result<'v, Self> {
        Ok(Dispatch("data"))
    }
}

#[post("/dispatch", data = "<field>")]
fn dispatch(field: Form<Dispatch>) -> &'static str {
    field.into_inner().0
}

#[test]
fn test_multipart_binary_without_content_type() {
    let client = Client::debug_with(routes![form, contents, dispatch]).unwrap();
    let binary: Vec<_> = (0..=255u8).collect();
    let values: [(&[u8], bool); 6] = [
        (&binary, false),
        ("héllo".as_bytes(), true),
        (b"\xef\xbb\xbf42", true),
        (b"\xff\xfe4\x002\x00", false),
        (b"\xfe\xff\x004\x002", false),
        (b"", true),
    ];

    let content_types = [None, Some("application/octet-stream"), Some("text/plain")];
    for filename in [None, Some("file.txt"), Some("")] {
        for content_type in content_types {
            let mut disposition = r#"name="file""#.to_string();
            if let Some(name) = filename {
                disposition.push_str(&format!("; filename=\"{name}\""));
            }
            if let Some(content_type) = content_type {
                disposition.push_str(&format!("\r\nContent-Type: {content_type}"));
            }

            for (value, utf8) in values {
                let data = filename.is_some() || content_type.is_some() || !utf8;
                for path in ["/", "/contents", "/dispatch"] {
                    let response = client.post(path)
                        .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
                        .body(multipart(&disposition, value)).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                    if path == "/contents" {
                        let field_type = match data {
                            true => content_type.unwrap_or("application/octet-stream"),
                            false => "",
                        };

                        assert_eq!(response.headers().get_one("X-On-Disk"),
                            Some(if data { "true" } else { "false" }));
                        assert_eq!(response.headers().get_one("X-File-Name"),
                            Some(filename.unwrap_or("<none>")));
                        assert_eq!(response.headers().get_one("X-Field-Type"),
                            Some(field_type));
                    }

                    if path == "/dispatch" {
                        assert_eq!(response.into_string().unwrap(),
                            if data { "data" } else { "value" });
                    } else {
                        assert_eq!(response.into_bytes().unwrap(), value);
                    }
                }
            }
        }
    }
}

#[test]
fn data_guard_limits_and_caps() {
    for binary_limit in [false, true] {
        let mut limits = Limits::default()
            .limit("file", 4.bytes()).limit("file/txt", 100.bytes())
            .limit("string", 4.bytes()).limit("bytes", 4.bytes());
        if binary_limit {
            limits = limits.limit("file", 100.bytes()).limit("file/bin", 4.bytes());
        }

        let config = rocket::Config { limits, ..rocket::Config::debug_default() };
        let app = rocket::custom(config).mount("/", routes![
            form, contents, capped_file, string, capped_bytes, capped_string,
        ]);

        let client = Client::debug(app).unwrap();
        let fields = [
            (r#"name="file"; filename="hello.txt""#, b"12345".as_slice()),
            (r#"name="file""#, b"\xff2345".as_slice()),
        ];

        let paths = [
            "/", "/contents", "/string",
            "/capped-file", "/capped-bytes", "/capped-string",
        ];

        for (disposition, value) in fields {
            for len in [4, 5] {
                for path in paths {
                    if path.contains("string") && std::str::from_utf8(value).is_err() {
                        continue;
                    }

                    let capped = path.starts_with("/capped");
                    let response = client.post(path)
                        .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
                        .body(multipart(disposition, &value[..len])).dispatch();
                    let expected = match capped || len == 4 {
                        true => Status::Ok,
                        false => Status::PayloadTooLarge,
                    };

                    assert_eq!(response.status(), expected, "{path}, {len} bytes");
                    if expected == Status::Ok {
                        if capped {
                            assert_eq!(response.headers().get_one("X-Complete"),
                                Some(if len == 4 { "true" } else { "false" }));
                        }
                        if path.contains("file") || path == "/contents" {
                            assert_eq!(response.headers().get_one("X-On-Disk"),
                                Some("true"));
                        }
                        assert_eq!(response.into_bytes().unwrap(), &value[..4]);
                    }
                }
            }
        }

        // Value fields use the `data-form` limit.
        let response = client.post("/string")
            .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
            .body(multipart(r#"name="value""#, b"12345")).dispatch();
        assert_eq!(response.into_string().unwrap(), "12345");
    }
}

#[test]
fn aggregate_multipart_limit() {
    for disposition in [r#"name="file""#, r#"name="file"; filename="file.bin""#] {
        let body = multipart(disposition, b"\xff\x00");
        for limit in [body.len(), body.len() - 1] {
            let config = rocket::Config {
                limits: Limits::default().limit("data-form", limit.bytes()),
                ..rocket::Config::debug_default()
            };
            let client = Client::debug(rocket::custom(config).mount("/", routes![form])).unwrap();
            let response = client.post("/")
                .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
                .body(body.clone()).dispatch();
            if limit == body.len() {
                assert_eq!(response.status(), Status::Ok);
                assert_eq!(response.into_bytes().unwrap(), b"\xff\x00");
            } else {
                assert!(response.status().class().is_client_error());
            }
        }
    }
}

#[test]
fn data_files_require_a_writable_temp_directory() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let mut directories = vec![missing];
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let readonly = dir.path().join("readonly");
        std::fs::create_dir(&readonly).unwrap();
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o500)).unwrap();
        match tempfile::NamedTempFile::new_in(&readonly) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                directories.push(readonly);
            }
            Ok(_) => {
                // A privileged process can write to a read-only directory.
                std::fs::set_permissions(&readonly,
                    std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            Err(e) => panic!("failed to probe temporary directory: {e}"),
        }
    }

    for directory in directories {
        let config = rocket::Config::figment().merge(("temp_dir", &directory));
        let client = Client::debug(rocket::custom(config).mount("/", routes![contents])).unwrap();
        let filename = r#"name="file"; filename="hello.txt""#;
        for (disposition, value, expected) in [
            (filename, b"hello".as_slice(), Status::BadRequest),
            (r#"name="file""#, b"\xff\x00", Status::BadRequest),
            (r#"name="file""#, b"hello", Status::Ok),
        ] {
            let response = client.post("/contents")
                .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
                .body(multipart(disposition, value)).dispatch();
            assert_eq!(response.status(), expected);
        }

        #[cfg(unix)]
        if directory.exists() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
}
