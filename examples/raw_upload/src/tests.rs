use rocket::local::blocking::Client;
use rocket::http::{Status, ContentType};

use std::env;
use std::io::Read;
use std::fs::{self, File};

const UPLOAD_CONTENTS: &str = "Hey! I'm going to be uploaded. :D Yay!";

#[test]
fn test_index() {
    let client = Client::new(super::rocket()).unwrap();
    let res = client.get("/").dispatch();
    assert_eq!(res.into_string(), Some(super::index().to_string()));
}

#[test]
fn test_raw_upload() {
    // Delete the upload file before we begin.
    let upload_file = env::temp_dir().join("upload.txt");
    let _ = fs::remove_file(&upload_file);

    // Do the upload. Make sure we get the expected results.
    let client = Client::new(super::rocket()).unwrap();
    let res = client.post("/upload")
        .header(ContentType::Plain)
        .body(UPLOAD_CONTENTS)
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_string(), Some(UPLOAD_CONTENTS.len().to_string()));

    // Ensure we find the body in the /tmp/upload.txt file.
    let mut file_contents = String::new();
    let mut file = File::open(&upload_file).expect("open upload.txt file");
    file.read_to_string(&mut file_contents).expect("read upload.txt");
    assert_eq!(&file_contents, UPLOAD_CONTENTS);
}
