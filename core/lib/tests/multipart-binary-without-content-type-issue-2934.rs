#[macro_use] extern crate rocket;

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;

#[post("/", data = "<form>")]
fn form(form: Form<&[u8]>) -> Vec<u8> {
    form.to_vec()
}

#[post("/file", data = "<file>")]
fn file(file: Form<TempFile<'_>>) -> (ContentType, String) {
    let content_type = file.content_type().unwrap().clone();
    let file_name = file.raw_name().unwrap().dangerous_unsafe_unsanitized_raw();
    (content_type, file_name.to_string())
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
fn test_multipart_binary_without_content_type() {
    let client = Client::debug_with(routes![form]).unwrap();
    let binary: Vec<_> = (0..=255u8).collect();
    for disposition in [r#"name="file"; filename="file.bin""#, r#"name="file""#] {
        for value in [binary.as_slice(), "héllo".as_bytes(), b"\xef\xbb\xbf42", b""] {
            let response = client.post("/")
                .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
                .body(multipart(disposition, value))
                .dispatch();

            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_bytes().unwrap(), value);
        }
    }
}

#[test]
fn test_multipart_file_without_content_type() {
    let client = Client::debug_with(routes![file]).unwrap();
    for file_name in ["file.txt", ""] {
        let disposition = format!(r#"name="file"; filename="{file_name}""#);
        let response = client.post("/file")
            .header("multipart/form-data; boundary=X".parse::<ContentType>().unwrap())
            .body(multipart(&disposition, b"hello"))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::Binary));
        assert_eq!(response.into_string().unwrap(), file_name);
    }
}

#[test]
fn test_multipart_number_without_content_type() {
    let client = Client::debug_with(routes![number]).unwrap();
    // A leading BOM is preserved, so it makes an otherwise valid number invalid.
    for (value, status) in [
        (&b"42"[..], Status::Ok),
        (&b"\xef\xbb\xbf42"[..], Status::UnprocessableEntity),
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
