#![no_main]

use rocket::http::uri::*;
use libfuzzer_sys::fuzz_target;

fn fuzz(data: &str) {
    if let Ok(uri) = Uri::parse_any(data) {
        match uri {
            Uri::Origin(uri) if uri.is_normalized() => {
                assert_eq!(uri.clone(), uri.into_normalized());
            }
            Uri::Absolute(uri) if uri.is_normalized() => {
                assert_eq!(uri.clone(), uri.into_normalized());
            }
            Uri::Reference(uri) if uri.is_normalized() => {
                assert_eq!(uri.clone(), uri.into_normalized());
            }
            _ => { /* not normalizable */ },
        }
    }
}

fuzz_target!(|data: &str| fuzz(data));
