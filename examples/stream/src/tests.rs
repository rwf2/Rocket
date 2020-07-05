use std::fs::{self, File};
use std::io::prelude::*;

use rocket::local::blocking::Client;

#[test]
fn test_root() {
    let client = Client::new(super::rocket()).unwrap();
    let res = client.get("/").dispatch();

    // Check that we have exactly 25,000 'a'.
    let res_str = res.into_string().unwrap();
    assert_eq!(res_str.len(), 25000);
    for byte in res_str.as_bytes() {
        assert_eq!(*byte, b'a');
    }
}

#[test]
fn test_file() {
    // Create the 'big_file'
    const CONTENTS: &str = "big_file contents...not so big here";
    let mut file = File::create(super::FILENAME).expect("create big_file");
    file.write_all(CONTENTS.as_bytes()).expect("write to big_file");

    // Get the big file contents, hopefully.
    let client = Client::new(super::rocket()).unwrap();
    let res = client.get("/big_file").dispatch();
    assert_eq!(res.into_string(), Some(CONTENTS.into()));

    // Delete the 'big_file'.
    fs::remove_file(super::FILENAME).expect("remove big_file");
}
