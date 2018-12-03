//! `Compression` fairing and `Compressed` responder to automatically and
//! on demand respectively compressing responses.
mod fairing;
mod responder;

pub use self::fairing::Compression;
pub use self::responder::Compressed;

crate use self::fairing::Context;
use rocket::http::hyper::header::{ContentEncoding, Encoding};
use rocket::{Request, Response};
use std::io::Read;

#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
use flate2::read::GzEncoder;

crate struct CompressionUtils;

impl CompressionUtils {
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

    fn skip_encoding(
        content_type: &Option<rocket::http::ContentType>,
        context: &rocket::State<Context>,
    ) -> bool {
        match content_type {
            Some(content_type) => context.exclusions.iter().any(|exc_media_type| {
                if exc_media_type.sub() == "*" {
                    *exc_media_type.top() == *content_type.top()
                } else {
                    *exc_media_type == *content_type.media_type()
                }
            }),
            None => false,
        }
    }

    fn compress_response(request: &Request, response: &mut Response, respect_excludes: bool) {
        if CompressionUtils::already_encoded(response) {
            return;
        }

        let content_type = response.content_type();

        if respect_excludes {
            let context = request
                .guard::<::rocket::State<Context>>()
                .expect("Compression Context registered in on_attach");

            if CompressionUtils::skip_encoding(&content_type, &context) {
                return;
            }
        }

        // Compression is done when the request accepts brotli or gzip encoding
        // and the corresponding feature is enabled
        if cfg!(feature = "brotli_compression") && CompressionUtils::accepts_encoding(request, "br")
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
        } else if cfg!(feature = "gzip_compression")
            && CompressionUtils::accepts_encoding(request, "gzip")
        {
            if let Some(plain) = response.take_body() {
                let compressor = GzEncoder::new(plain.into_inner(), flate2::Compression::default());

                CompressionUtils::set_body_and_encoding(response, compressor, Encoding::Gzip);
            }
        }
    }
}
