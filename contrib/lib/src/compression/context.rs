//! Automatic compression exclusions struct and default exclusions.

use rocket::http::MediaType;

crate struct Context {
    crate exclusions: Vec<MediaType>,
}

impl Context {
    crate fn new() -> Context {
        Context {
            exclusions: vec![
                MediaType::parse_flexible("application/gzip").unwrap(),
                MediaType::parse_flexible("application/zip").unwrap(),
                MediaType::parse_flexible("image/*").unwrap(),
                MediaType::parse_flexible("video/*").unwrap(),
                MediaType::parse_flexible("application/wasm").unwrap(),
                MediaType::parse_flexible("application/octet-stream").unwrap(),
            ],
        }
    }
    crate fn with_exclusions(excls: Vec<MediaType>) -> Context {
        Context { exclusions: excls }
    }
}
