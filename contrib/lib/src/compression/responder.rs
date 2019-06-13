use rocket::response::{self, Responder, Response};
use rocket::Request;

use super::CompressionUtils;

/// Compresses responses with Brotli or Gzip compression.
///
/// The `Compress` type implements brotli and gzip compression for responses in
/// accordance with the `Accept-Encoding` header. If accepted, brotli
/// compression is preferred over gzip.
///
/// In the brotli compression mode (using the
/// [rust-brotli](https://github.com/dropbox/rust-brotli) crate), quality is set
/// to 2 in order to achieve fast compression with a compression ratio similar
/// to gzip. When appropriate, brotli's text and font compression modes are
/// used.
///
/// In the gzip compression mode (using the
/// [flate2](https://github.com/alexcrichton/flate2-rs) crate), quality is set
/// to the default (9) in order to have good compression ratio.
///
/// Responses that already have a `Content-Encoding` header are not compressed.
///
/// # Usage
///
/// Compress responses by wrapping a `Responder` inside `Compress`:
///
/// ```rust
/// use rocket_contrib::compression::Compress;
///
/// # #[allow(unused_variables)]
/// let response = Compress("Hi.");
/// ```
#[derive(Debug)]
pub struct Compress<R>(pub R);

impl<'r, R: Responder<'r>> Responder<'r> for Compress<R> {
    #[inline(always)]
    fn respond_to(self, request: &Request<'_>) -> response::Result<'r> {
        let mut response = Response::build()
            .merge(self.0.respond_to(request)?)
            .finalize();

        CompressionUtils::compress_response(request, &mut response, &[]);
        Ok(response)
    }
}
