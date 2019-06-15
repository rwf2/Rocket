//! Gzip and Brotli response compression.
//!
//! See the [`Compression`](compression::Compression) and
//! [`Compress`](compression::Compress) types for further details.
//!
//! # Enabling
//!
//! This module is only available when one of the `brotli_compression`,
//! `gzip_compression`, or `compression` features is enabled. Enable
//! one of these in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["compression"]
//! ```
//!
//! # Security Implications
//!
//! In some cases, HTTP compression on a site served over HTTPS can make a web
//! application vulnerable to attacks including BREACH. These risks should be
//! evaluated in the context of your application before enabling compression.
//!

mod fairing;
mod responder;

pub use self::fairing::Compression;
pub use self::responder::Compress;

use std::io::Read;

use rocket::http::MediaType;
use rocket::http::hyper::header::{ContentEncoding, Encoding};
use rocket::{Request, Response};

#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
use flate2::read::GzEncoder;

struct CompressionUtils;

impl CompressionUtils {
    fn accepts_encoding(request: &Request<'_>, encoding: &str) -> bool {
        request
            .headers()
            .get("Accept-Encoding")
            .flat_map(|accept| accept.split(','))
            .map(|accept| accept.trim())
            .any(|accept| accept == encoding)
    }

    fn already_encoded(response: &Response<'_>) -> bool {
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

    fn skip_encoding(
        content_type: &Option<rocket::http::ContentType>,
        exclusions: &[MediaType],
    ) -> bool {
        match content_type {
            Some(content_type) => exclusions.iter().any(|exc_media_type| {
                if exc_media_type.sub() == "*" {
                    *exc_media_type.top() == *content_type.top()
                } else {
                    *exc_media_type == *content_type.media_type()
                }
            }),
            None => false,
        }
    }

    fn compress_response(request: &Request<'_>, response: &mut Response<'_>, exclusions: &[MediaType]) {
        if CompressionUtils::already_encoded(response) {
            return;
        }

        let content_type = response.content_type();

        if CompressionUtils::skip_encoding(&content_type, exclusions) {
            return;
        }

        // Compression is done when the request accepts brotli or gzip encoding
        // and the corresponding feature is enabled
        if cfg!(feature = "brotli_compression") && CompressionUtils::accepts_encoding(request, "br")
        {
            #[cfg(feature = "brotli_compression")]
            {
                if let Some(plain) = response.take_body() {
                    let content_type_top = content_type.as_ref().map(|ct| ct.top());
                    let mut params = brotli::enc::BrotliEncoderInitParams();
                    params.quality = 2;
                    if content_type_top == Some("text".into()) {
                        params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                    } else if content_type_top == Some("font".into()) {
                        params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                    }

                    let compressor =
                        brotli::CompressorReader::with_params(plain.into_inner(), 4096, &params);

                    CompressionUtils::set_body_and_encoding(
                        response,
                        compressor,
                        Encoding::EncodingExt("br".into()),
                    );
                }
            }
        } else if cfg!(feature = "gzip_compression")
            && CompressionUtils::accepts_encoding(request, "gzip")
        {
            #[cfg(feature = "gzip_compression")]
            {
                if let Some(plain) = response.take_body() {
                    let compressor = GzEncoder::new(plain.into_inner(), flate2::Compression::default());

                    CompressionUtils::set_body_and_encoding(response, compressor, Encoding::Gzip);
                }
            }
        }
    }
}
