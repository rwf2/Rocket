//! This module provides brotli ang gzip compression for all non-image
//! responses for requests that send Accept-Encoding br and gzip. If
//! accepted, brotli compression is preferred over gzip.
//!
//! To add this feature to your Rocket application, use
//! .attach(rocket_contrib::Compression::fairing())
//! to your Rocket instance. Note that you must add the
//! "compression" feature for brotli and gzip compression to your rocket_contrib
//! dependency in Cargo.toml. Additionally, you can load only brotli compression
//! using "brotli_compression" feature or load only gzip compression using
//! "gzip_compression" in your rocket_contrib dependency in Cargo.toml.
//!
//! In the brotli algorithm, quality is set to 2 in order to have really fast
//! compressions with compression ratio similar to gzip. Also, text and font
//! compression mode is set regarding the Content-Type of the response.
//!
//! In the gzip algorithm, quality is the default (9) in order to have good
//! compression ratio.
//!
//! For brotli compression, the rust-brotli crate is used.
//! For gzip compression, flate2 crate is used.

use std::io::Cursor;
#[cfg(feature = "gzip_compression")]
use std::io::Write;

use rocket::{Request, Response};
use rocket::http::Header;
use rocket::fairing::{Fairing, Info, Kind};

#[cfg(feature = "brotli_compression")]
use brotli;
#[cfg(feature = "brotli_compression")]
use brotli::enc::backward_references::BrotliEncoderMode;

#[cfg(feature = "gzip_compression")]
use flate2;
#[cfg(feature = "gzip_compression")]
use flate2::write::GzEncoder;

#[derive(Debug, Default)]
pub struct Compression;

impl Compression {
    /// This function creates a Compression to be used in your Rocket
    /// instance. Add ```.attach(rocket_contrib::Compression::fairing())```
    /// to your Rocket instance to use this fairing.
    ///
    /// # Returns
    ///
    /// A Compression instance.
    pub fn fairing() -> Compression {
        Compression::default()
    }
}

impl Fairing for Compression {
    fn info(&self) -> Info {
        Info {
            name: "Brotli and gzip compressors for responses",
            kind: Kind::Response,
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        let content_type = response.content_type();
        // Images must not be compressed
        if let Some(ref content_type) = content_type {
            if content_type.top() == "image" {
                return;
            }
        }

        // The compression is done if the request supports brotli or gzip and
        // the corresponding feature is enabled
        if cfg!(feature = "brotli_compression")
            && request
                .headers()
                .get("Accept-Encoding")
                .flat_map(|e| e.trim().split(","))
                .any(|e| e.trim() == "br" || e.trim() == "brotli")
        {
            #[cfg(feature = "brotli_compression")]
            {
                let body = response.body_bytes();
                if let Some(body) = body {
                    let mut plain = Cursor::new(body);
                    let mut compressed = Cursor::new(Vec::<u8>::new());
                    let mut params = brotli::enc::BrotliEncoderInitParams();
                    params.quality = 2;
                    if let Some(ref content_type) = content_type {
                        if content_type.top() == "text" {
                            params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                        } else if content_type.top() == "font" {
                            params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                        }
                    }
                    if brotli::BrotliCompress(&mut plain, &mut compressed, &params).is_ok() {
                        response.remove_header("Content-Encoding");
                        response.adjoin_header(Header::new("Content-Encoding", "br"));
                        response.set_sized_body(compressed);
                    }
                }
            }
        } else if cfg!(feature = "gzip_compression")
            && request
                .headers()
                .get("Accept-Encoding")
                .flat_map(|e| e.trim().split(","))
                .any(|e| e.trim() == "gzip")
        {
            #[cfg(feature = "gzip_compression")]
            {
                let body = response.body_bytes();
                if let Some(body) = body {
                    let mut compressed = Cursor::new(Vec::<u8>::new());
                    if GzEncoder::new(&mut compressed, flate2::Compression::default())
                        .write_all(&body)
                        .is_ok()
                    {
                        response.remove_header("Content-Encoding");
                        response.adjoin_header(Header::new("Content-Encoding", "gzip"));
                        response.set_sized_body(compressed);
                    }
                }
            }
        }
    }
}
