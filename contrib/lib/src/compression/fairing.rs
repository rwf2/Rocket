//! Automatic response compression.
//!
//! See the [`Compression`](compression::fairing::Compression) type for further
//! details.

use rocket::config::{ConfigError, Value};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::MediaType;
use rocket::Rocket;
use rocket::{Request, Response};

crate use super::CompressionUtils;

crate struct Context {
    crate exclusions: Vec<MediaType>,
}

impl Context {
    crate fn new() -> Context {
        Context {
            exclusions: vec![
                MediaType::parse_flexible("application/gzip").unwrap(),
                MediaType::parse_flexible("application/zip").unwrap(),
                MediaType::parse_flexible("image/*").unwrap(),
                MediaType::parse_flexible("video/*").unwrap(),
                MediaType::parse_flexible("application/wasm").unwrap(),
                MediaType::parse_flexible("application/octet-stream").unwrap(),
            ],
        }
    }
    crate fn with_exclusions(excls: Vec<MediaType>) -> Context {
        Context { exclusions: excls }
    }
}

/// The Compression type implements brotli and gzip compression for responses in
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
/// to the default (9) in order to have good compression ratio.
///
/// This fairing does not compress responses that already have a
/// `Content-Encoding` header.
///
/// This fairing ignores the responses with a `Content-Type` matching any of
/// the following default types:
///
/// - application/gzip
/// - application/brotli
/// - image/*
/// - video/*
/// - application/wasm
/// - application/octet-stream
///
/// The excluded types can be changed changing the `compress.exclude` Rocket
/// configuration property.
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
        match rocket.config().get_table("compress").and_then(|t| {
            t.get("exclude")
                .ok_or(ConfigError::Missing(String::from("exclude")))
        }) {
            Ok(excls) => match excls.as_array() {
                Some(excls) => {
                    let mut error = false;
                    let mut exclusions_vec = Vec::with_capacity(excls.len());
                    for e in excls {
                        match e {
                            Value::String(s) => match MediaType::parse_flexible(s) {
                                Some(media_type) => exclusions_vec.push(media_type),
                                None => {
                                    error = true;
                                    warn_!(
                                "Exclusions must be valid content types, using default compression exclusions '{:?}'",
                                ctxt.exclusions
                            );
                                    break;
                                }
                            },
                            _ => {
                                error = true;
                                warn_!(
                                "Exclusions must be strings, using default compression exclusions '{:?}'",
                                ctxt.exclusions
                            );
                                break;
                            }
                        }
                    }
                    if !error {
                        ctxt = Context::with_exclusions(exclusions_vec);
                    }
                }
                None => {
                    warn_!(
                                "Exclusions must be an array of strings, using default compression exclusions '{:?}'",
                                ctxt.exclusions
                            );
                }
            },
            Err(ConfigError::Missing(_)) => { /* ignore missing */ }
            Err(e) => {
                e.pretty_print();
                warn_!(
                    "Using default compression exclusions '{:?}'",
                    ctxt.exclusions
                );
            }
        };

        Ok(rocket.manage(ctxt))
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        CompressionUtils::compress_response(request, response, true);
    }
}
