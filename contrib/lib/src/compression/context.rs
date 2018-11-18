//! Automatic compression exclusions struct and default exclusions.

crate struct Context {
    crate exclusions: Vec<String>,
}

impl Context {
    crate fn new() -> Context {
        Context {
            exclusions: vec![
                String::from("application/gzip"),
                String::from("application/brotli"),
                String::from("application/zip"),
                String::from("image/*"),
                String::from("application/wasm"),
                String::from("application/binary"),
            ],
        }
    }
    crate fn with_exclusions(excls: Vec<String>) -> Context {
        Context { exclusions: excls }
    }
}
