use rocket::http::hyper::header::Encoding;
use rocket::response::{self, Responder, Response};
use rocket::Request;

#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
use flate2::read::GzEncoder;

crate use super::CompressionUtils;

#[derive(Debug)]
pub struct Compressed<'a>(Response<'a>);

impl<'a> Compressed<'a> {
    pub fn new(response: Response<'a>) -> Compressed<'a> {
        Compressed { 0: response }
    }
}

/// Serializes the value into JSON. Returns a response with Content-Type JSON
/// and a fixed-size body with the serialized value.
impl<'a> Responder<'a> for Compressed<'a> {
    #[inline]
    fn respond_to(self, request: &Request) -> response::Result<'a> {
        if CompressionUtils::already_encoded(&self.0) {
            return Ok(self.0);
        }

        let mut response = Response::build().merge(self.0).finalize();

        // Do not compress configured types exceptions
        let content_type = response.content_type();
        let content_type_top = content_type.as_ref().map(|ct| ct.top());

        // Compression is done when the request accepts brotli or gzip encoding
        // and the corresponding feature is enabled
        if cfg!(feature = "brotli_compression") && CompressionUtils::accepts_encoding(request, "br")
        {
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

                CompressionUtils::set_body_and_encoding(
                    &mut response,
                    compressor,
                    Encoding::EncodingExt("br".into()),
                );
            }
        } else if cfg!(feature = "gzip_compression")
            && CompressionUtils::accepts_encoding(request, "gzip")
        {
            if let Some(plain) = response.take_body() {
                let compressor = GzEncoder::new(plain.into_inner(), flate2::Compression::default());

                CompressionUtils::set_body_and_encoding(&mut response, compressor, Encoding::Gzip);
            }
        }
        Ok(response)
    }
}
