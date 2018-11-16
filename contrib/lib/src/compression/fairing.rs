//! Automatic response compression.
//!
//! See the [`Compression`](compression::Compression) type for further details.

use rocket::config::{ConfigError, Value};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::hyper::header::Encoding;
use rocket::Rocket;
use rocket::{Request, Response};

#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
use flate2::read::GzEncoder;

crate use super::context::{Context, ContextManager};
crate use super::CompressionUtils;

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
}

impl Fairing for Compression {
    fn info(&self) -> Info {
        Info {
            name: "Response compression",
            kind: Kind::Attach | Kind::Response,
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        let mut ctxt = Context::new();
        match rocket.config().get_slice("compress.exclude") {
            Ok(excps) => {
                let mut error = false;
                let mut exceptions_vec = Vec::with_capacity(excps.len());
                for e in excps {
                    match e {
                        Value::String(s) => exceptions_vec.push(s.clone()),
                        _ => {
                            error = true;
                            warn_!(
                                "Exceptions must be strings, using default compression exceptions '{:?}'",
                                ctxt.exceptions
                            );
                            break;
                        }
                    }
                }
                if !error {
                    ctxt = Context::with_exceptions(exceptions_vec);
                }
            }
            Err(ConfigError::Missing(_)) => { /* ignore missing */ }
            Err(e) => {
                e.pretty_print();
                warn_!(
                    "Using default compression exceptions '{:?}'",
                    ctxt.exceptions
                );
            }
        };

        Ok(rocket.manage(ContextManager::new(ctxt)))
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if CompressionUtils::already_encoded(response) {
            return;
        }

        let cm = request
            .guard::<::rocket::State<ContextManager>>()
            .expect("Compression ContextManager registered in on_attach");

        // Do not compress configured types exceptions
        let content_type = response.content_type();
        let content_type_top = content_type.as_ref().map(|ct| ct.top());
        if CompressionUtils::skip_encoding(&content_type, &content_type_top, &cm) {
            return;
        }

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
