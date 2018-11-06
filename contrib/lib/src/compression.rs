//! Automatic response compression.
//!
//! See the [`Compression`](compression::Compression) type for further details.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::hyper::header::{ContentEncoding, Encoding};
use rocket::{Request, Response};
use std::io::Read;

#[cfg(feature = "brotli_compression")]
extern crate brotli;
#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
extern crate flate2;
#[cfg(feature = "gzip_compression")]
use flate2::read::GzEncoder;

/// accordance with the Accept-Encoding header. If accepted, brotli compression
/// is preferred over gzip.
///
/// In the brotli compression mode (using the
/// [rust-brotli](https://github.com/dropbox/rust-brotli) crate), quality is set
/// to 2 in order to achieve fast compression with a compression ratio similar
/// to gzip. When appropriate, brotli's text and font compression modes are
/// used.
///
/// In the gzip compression mode (using the
/// [flate2](https://github.com/alexcrichton/flate2-rs) crate), quality is set
/// The Compression type implements brotli and gzip compression for responses in
/// to the default (9) in order to have good compression ratio.
///
/// This fairing does not compress responses with a `Content-Type` matching
/// `image/*`, nor does it compress responses that already have a
/// `Content-Encoding` header.
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
/// Then, ensure that the compression [fairing](/rocket/fairing/) is attached to
/// your Rocket application:
///
/// ```rust
/// extern crate rocket;
/// extern crate rocket_contrib;
///
/// use rocket_contrib::compression::Compression;
///
/// fn main() {
///     rocket::ignite()
///         // ...
///         .attach(Compression::fairing())
///         // ...
///     # ;
/// }
/// ```
pub struct Compression(());

impl Compression {
    /// Returns a fairing that compresses outgoing requests.
    ///
    /// ## Example
    /// To attach this fairing, simply call `attach` on the application's
    /// `Rocket` instance with `Compression::fairing()`:
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_contrib;
    ///
    /// use rocket_contrib::compression::Compression;
    ///
    /// fn main() {
    ///     rocket::ignite()
    ///         // ...
    ///         .attach(Compression::fairing())
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn fairing() -> Compression {
        Compression { 0: () }
    }

    fn accepts_encoding(request: &Request, encoding: &str) -> bool {
        request
            .headers()
            .get("Accept-Encoding")
            .flat_map(|accept| accept.split(","))
            .map(|accept| accept.trim())
            .any(|accept| accept == encoding)
    }

    fn already_encoded(response: &Response) -> bool {
        response.headers().get("Content-Encoding").next().is_some()
    }

    fn set_body_and_encoding<'r, B: Read + 'r>(
        response: &mut Response<'r>,
        body: B,
        encoding: Encoding,
    ) {
        response.set_header(ContentEncoding(vec![encoding]));
        response.set_streamed_body(body);
    }
}

impl Fairing for Compression {
    fn info(&self) -> Info {
        Info {
            name: "Response compression",
            kind: Kind::Response,
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if Compression::already_encoded(response) {
            return;
        }

        let content_type = response.content_type();
        let content_type_top = content_type.as_ref().map(|ct| ct.top());
        // Do not compress images
        if content_type_top == Some("image".into()) {
            return;
        }

        // Compression is done when the request accepts brotli or gzip encoding
        // and the corresponding feature is enabled
        if cfg!(feature = "brotli_compression") && Compression::accepts_encoding(request, "br") {
            if let Some(plain) = response.take_body() {
                let mut params = brotli::enc::BrotliEncoderInitParams();
                params.quality = 2;
                if content_type_top == Some("text".into()) {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                } else if content_type_top == Some("font".into()) {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                }

                let compressor =
                    brotli::CompressorReader::with_params(plain.into_inner(), 4096, &params);

                Compression::set_body_and_encoding(
                    response,
                    compressor,
                    Encoding::EncodingExt("br".into()),
                );
            }
        } else if cfg!(feature = "gzip_compression")
            && Compression::accepts_encoding(request, "gzip")
        {
            if let Some(plain) = response.take_body() {
                let compressor = GzEncoder::new(plain.into_inner(), flate2::Compression::default());

                Compression::set_body_and_encoding(response, compressor, Encoding::Gzip);
            }
        }
    }
}
