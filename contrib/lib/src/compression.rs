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
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Read, Write};

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

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

    fn accepts_encoding(request: &Request, encodings: &[&str]) -> bool {
        request
            .headers()
            .get("Accept-Encoding")
            .flat_map(|e| e.split(","))
            .any(|e| encodings.contains(&e.trim()))
    }

    fn set_body_and_header<'h, B>(response: &mut Response<'h>, mut body: B, header: &'h str)
    where
        B: Read + Seek + 'h,
    {
        if body.seek(SeekFrom::Start(0)).is_ok() {
            response.remove_header("Content-Encoding");
            response.adjoin_header(Header::new("Content-Encoding", header));
            response.set_streamed_body(body);
        }
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
            && Compression::accepts_encoding(request, &["br", "brotli"])
        {
            let mut compressed = Cursor::new(Vec::<u8>::new());
            if match response.body() {
                Some(plain) => {
                    let mut params = brotli::enc::BrotliEncoderInitParams();
                    params.quality = 2;
                    if let Some(ref content_type) = content_type {
                        if content_type.top() == "text" {
                            params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                        } else if content_type.top() == "font" {
                            params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                        }
                    }
                    brotli::BrotliCompress(&mut plain.into_inner(), &mut compressed, &params)
                        .is_ok()
                }
                None => false,
            } {
                Compression::set_body_and_header(response, compressed, "br");
            }
        } else if cfg!(feature = "gzip_compression")
            && Compression::accepts_encoding(request, &["gzip"])
        {
            let mut gz = GzEncoder::new(
                Cursor::new(Vec::<u8>::with_capacity(100)),
                flate2::Compression::default(),
            );
            if match response.body() {
                Some(plain) => gz.write(&plain.into_bytes().unwrap()).is_ok(),
                None => false,
            } {
                Compression::set_body_and_header(response, gz.finish().unwrap(), "gzip");
            }
        }
    }
}
