#![no_main]
use libfuzzer_sys::fuzz_target;
use bytes::Bytes;
use http::Uri;

fuzz_target!(|data: &[u8]| {
    // Rocket URI/route parsing — pre-auth
    let _ = Uri::from_maybe_shared(Bytes::copy_from_slice(data));
});
