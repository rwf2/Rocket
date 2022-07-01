#![no_main]

use rocket::http::uri::*;
use libfuzzer_sys::fuzz_target;

fn fuzz(data: &str) {
    // Fuzz the top-level parser.
    if let Ok(uri) = Uri::parse_any(data) {
        // Ensure Uri::parse::<T>() => T::parse().
        match uri {
            Uri::Asterisk(_) => { Asterisk::parse(data).expect("Asterisk"); },
            Uri::Origin(_) => { Origin::parse(data).expect("Origin"); },
            Uri::Authority(_) => { Authority::parse(data).expect("Authority"); },
            Uri::Absolute(_) => { Absolute::parse(data).expect("Absolute"); },
            Uri::Reference(_) => { Reference::parse(data).expect("Reference"); },
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = std::str::from_utf8(data).map(fuzz);
});
