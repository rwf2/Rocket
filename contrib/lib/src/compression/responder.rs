//! Response on demand compression.
//!
//! See the [`Compression`](compression::responder::Compressed) type for
//! further details.

use rocket::response::{self, Responder, Response};
use rocket::Request;

crate use super::CompressionUtils;

/// Compress a `Responder` response ignoring the compression exclusions.
///
/// Delegates the remainder of the response to the wrapped `Responder`.
///
/// # Usage
///
/// To use, add the `brotli_compression` feature, the `gzip_compression`
/// feature, or the `compression` feature (to enable both algorithms) to the
/// `rocket_contrib` dependencies section of your `Cargo.toml`:
///
/// ```toml,ignore
/// [dependencies.rocket_contrib]
/// version = "*"
/// default-features = false
/// features = ["compression"]
/// ```
///
/// Then, compress the desired response wrapping a `Responder` inside
/// `Compressed`:
///
/// ```rust
/// use rocket_contrib::compression::Compressed;
///
/// # #[allow(unused_variables)]
/// let response = Compressed("Hi.");
/// ```
#[derive(Debug)]
pub struct Compressed<R>(pub R);

impl<'r, R: Responder<'r>> Compressed<R> {
    pub fn new(response: R) -> Compressed<R> {
        Compressed { 0: response }
    }
}

impl<'r, R: Responder<'r>> Responder<'r> for Compressed<R> {
    #[inline(always)]
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        let mut response = Response::build()
            .merge(self.0.respond_to(request)?)
            .finalize();

        CompressionUtils::compress_response(request, &mut response, false);
        Ok(response)
    }
}
