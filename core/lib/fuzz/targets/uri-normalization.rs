#![no_main]

use rocket::http::uri::Uri;
use libfuzzer_sys::fuzz_target;

fn fuzz(data: &str) {
    if let Ok(uri) = Uri::parse_any(data) {
        if uri.is_normalized() {
            assert_eq!(uri.clone(), uri.into_normalized());
        }
    }
}

fuzz_target!(|data: &str| fuzz(data));
