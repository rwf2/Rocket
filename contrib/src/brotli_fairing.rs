//! This module provides brotli compression for all non-image responses without
//! Content-Encoding header or with Content-Encoding br for requests that send
//! Accept-Encoding br.
//!
//! To add this feature to your Rocket application, use
//! .attach(rocket_contrib::BrotliFairing::fairing())
//! to your Rocket instance. Note that you must add the feature
//! "brotli_fairing" to your rocket_contrib dependency in Cargo.toml.
//!
//! Quality is set to 2 in order to have really fast compressions with
//! compression ratio similar to gzip. Text and font compression mode is set
//! regarding the Content-Type of the response.

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};

use rocket::{Data, Request, Response};
use rocket::http::Header;
use rocket::fairing::{Fairing, Info, Kind};

use brotli;
use brotli::enc::backward_references::BrotliEncoderMode;

#[derive(Debug, Default)]
pub struct BrotliFairing {
    support: AtomicBool,
}

impl BrotliFairing {
    /// This function creates a BrotliFairing to be used in your Rocket
    /// instance. Add ```.attach(rocket_contrib::BrotliFairing::fairing())```
    /// to your Rocket instance to use this fairing.
    ///
    /// # Returns
    ///
    /// A BrotliFairing instance.
    pub fn fairing() -> BrotliFairing {
        Default::default()
    }
}

impl Fairing for BrotliFairing {
    fn info(&self) -> Info {
        Info {
            name: "Brotli compressor for responses",
            kind: Kind::Request | Kind::Response,
        }
    }

    fn on_request(&self, request: &mut Request, _: &Data) {
        if request
            .headers()
            .get("Accept-Encoding")
            .any(|x| x.contains("br"))
        {
            self.support.store(true, Ordering::Relaxed);
        } else {
            self.support.store(false, Ordering::Relaxed);
        }
    }

    fn on_response(&self, _request: &Request, response: &mut Response) {
        let mut content_header = false;
        let mut content_header_br = false;
        let mut image = false;
        let headers = response.headers().clone();

        if headers.contains("Content-Encoding") {
            content_header = true;
            if headers.get("Content-Encoding").any(|x| x == "br") {
                content_header_br = true;
            }
        }
        if headers.get("Content-Type").any(|x| x.contains("image/")) {
            image = true;
        }

        // The compression is done if the request supports brotli, the response
        // does not have any Content-Encoding header or the Content-Encoding is
        // brotli and the Content-Type is not an image (images compression
        // ratio is minimum)
        if self.support.load(Ordering::Relaxed) &&
            (!content_header || content_header_br) &&
            !image
        {
            response.adjoin_header(Header::new("Content-Encoding", "br"));
            let body = response.body_bytes();
            if let Some(body) = body {
                let mut plain = Cursor::new(body);
                let mut compressed = Cursor::new(Vec::<u8>::new());
                let mut params = brotli::enc::BrotliEncoderInitParams();
                params.quality = 2;
                let content_type = headers
                    .get("Content-Type")
                    .collect::<Vec<&str>>()
                    .join(", ");
                if content_type.contains("text/") {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
                } else if content_type.contains("font/") {
                    params.mode = BrotliEncoderMode::BROTLI_MODE_FONT;
                }
                if brotli::BrotliCompress(&mut plain, &mut compressed, &params)
                    .is_ok()
                {
                    response.set_sized_body(compressed);
                }
            }
        } else if content_header_br {
            // if the request does not accept br and the Content-Encoding br is
            // present, it must be removed
            response.remove_header("Content-Encoding");

            for enc in headers
                .into_iter()
                .filter(|x| x.name() == "Content-Encoding" && x.value() != "br")
            {
                let header = enc.clone();
                response.adjoin_header(header);
            }
        }
    }
}
