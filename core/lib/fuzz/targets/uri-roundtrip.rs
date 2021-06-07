#![no_main]

use rocket::http::uri::*;
use libfuzzer_sys::fuzz_target;

fn fuzz(data: &str) {
    if let Ok(uri) = Uri::parse_any(data) {
        let string = uri.to_string();
        let _ = match uri {
            Uri::Asterisk(_) => Asterisk::parse_owned(string).expect("Asterisk").to_string(),
            Uri::Origin(_) => Origin::parse_owned(string).expect("Origin").to_string(),
            Uri::Authority(_) => Authority::parse_owned(string).expect("Authority").to_string(),
            Uri::Absolute(_) => Absolute::parse_owned(string).expect("Absolute").to_string(),
            Uri::Reference(_) => Reference::parse_owned(string).expect("Reference").to_string(),
        };
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = std::str::from_utf8(data).map(fuzz);
});
