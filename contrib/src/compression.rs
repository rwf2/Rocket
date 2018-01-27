//! This module provides brotli ang gzip compression for all non-image
//! responses without Content-Encoding header or with Content-Encoding br or
//! gzip respectively for requests that send Accept-Encoding br and gzip. If
//! accepted, brotli compression is preferred over gzip.
//!
//! To add this feature to your Rocket application, use
//! .attach(rocket_contrib::Compression::fairing())
//! to your Rocket instance. Note that you must add the feature
//! "compression" to your rocket_contrib dependency in Cargo.toml.
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

use std::io::{Cursor, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use rocket::{Data, Request, Response};
use rocket::http::{Header, HeaderMap};
use rocket::fairing::{Fairing, Info, Kind};

use brotli;
use brotli::enc::backward_references::BrotliEncoderMode;

use flate2;
use flate2::write::GzEncoder;

#[derive(Debug, Default)]
pub struct Compression {
    accepts_brotli: AtomicBool,
    accepts_gzip: AtomicBool,
}

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
            kind: Kind::Request | Kind::Response,
        }
    }

    fn on_request(&self, request: &mut Request, _: &Data) {
        let accept_headers: Vec<&str> =
            request.headers().get("Accept-Encoding").collect();
        if accept_headers.iter().any(|x| x.contains("br")) {
            self.accepts_brotli.store(true, Ordering::Relaxed);
        } else {
            self.accepts_brotli.store(false, Ordering::Relaxed);
        }
        if accept_headers.iter().any(|x| x.contains("gzip")) {
            self.accepts_gzip.store(true, Ordering::Relaxed);
        } else {
            self.accepts_gzip.store(false, Ordering::Relaxed);
        }
    }

    fn on_response(&self, _request: &Request, response: &mut Response) {
        let mut content_header = false;
        let mut content_header_br = false;
        let mut content_header_gzip = false;
        let mut brotli_compressed = false;
        let mut gzip_compressed = false;
        let mut image = false;
        let headers = response.headers().clone();

        if headers.contains("Content-Encoding") {
            content_header = true;
            if headers.get("Content-Encoding").any(|x| x == "br") {
                content_header_br = true;
            }
            if headers.get("Content-Encoding").any(|x| x == "gzip") {
                content_header_gzip = true;
            }
        }
        if headers.get("Content-Type").any(|x| x.contains("image/")) {
            image = true;
        }
        // The compression is done if the request supports brotli, the response
        // does not have any Content-Encoding header or the Content-Encoding is
        // brotli and the Content-Type is not an image (images compression
        // ratio is minimum)
        if self.accepts_brotli.load(Ordering::Relaxed) &&
           (!content_header || content_header_br) && !image {
            brotli_compressed = true;
            response.adjoin_header(Header::new("Content-Encoding", "br"));
            let body = response.body_bytes();
            if let Some(body) = body {
                let mut plain = Cursor::new(body);
                let mut compressed = Cursor::new(Vec::<u8>::new());
                let mut params = brotli::enc::BrotliEncoderInitParams();
                params.quality = 2;
                let content_type = headers.get("Content-Type")
                    .collect::<Vec<&str>>()
                    .join(", ");
                if content_type.contains("text/") {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                } else if content_type.contains("font/") {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                }
                if brotli::BrotliCompress(&mut plain, &mut compressed, &params).is_ok() {
                    response.set_sized_body(compressed);
                }
            }
        } else if self.accepts_gzip.load(Ordering::Relaxed) &&
           (!content_header || content_header_gzip || content_header_br) &&
           !image {
            gzip_compressed = true;
            response.adjoin_header(Header::new("Content-Encoding", "gzip"));
            let body = response.body_bytes();
            if let Some(body) = body {
                let mut compressed = Cursor::new(Vec::<u8>::new());
                if GzEncoder::new(&mut compressed, flate2::Compression::default())
                    .write_all(&body)
                    .is_ok() {
                    response.set_sized_body(compressed);
                }
            }
        }

        // if the response has not been compressed with an algorithm and the
        // Content-Encoding is present, it must be removed
        if content_header_br && !brotli_compressed {
            Compression::remove_content_header("br",
                                               response.headers().clone(),
                                               response);
        }
        if content_header_gzip && !gzip_compressed {
            Compression::remove_content_header("gzip",
                                               response.headers().clone(),
                                               response);
        }
    }
}

impl Compression {
    fn remove_content_header<'a, 'b: 'a>(to_delete: &'a str,
                                         headers: HeaderMap<'b>,
                                         response: &mut Response<'a>) {
        response.remove_header("Content-Encoding");
        for enc in headers.into_iter()
            .filter(|x| x.name() == "Content-Encoding" && x.value() != to_delete) {
            let header = enc.clone();
            println!("{:?}", header);
            response.adjoin_header(header);
        }
    }
}
