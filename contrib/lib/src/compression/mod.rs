mod context;
mod fairing;
mod responder;

pub use self::fairing::Compression;
pub use self::responder::Compressed;

crate use self::context::ContextManager;
use rocket::http::hyper::header::{ContentEncoding, Encoding};
use rocket::http::uncased::UncasedStr;
use rocket::http::MediaType;
use rocket::{Request, Response};
use std::io::Read;

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
        content_type_top: &Option<&UncasedStr>,
        cm: &rocket::State<ContextManager>,
    ) -> bool {
        let context = cm.context();
        let exceptions = context.exceptions();
        exceptions
            .iter()
            .filter(|c| match content_type {
                Some(ref orig_content_type) => match MediaType::parse_flexible(c) {
                    Some(exc_media_type) => {
                        if exc_media_type.sub() == "*" {
                            Some(exc_media_type.top()) == *content_type_top
                        } else {
                            exc_media_type == *orig_content_type.media_type()
                        }
                    }
                    None => {
                        if c.contains("/") {
                            let split: Vec<&str> = c.split("/").collect();

                            let exc_media_type =
                                MediaType::new(String::from(split[0]), String::from(split[1]));

                            if split[1] == "*" {
                                Some(exc_media_type.top()) == *content_type_top
                            } else {
                                exc_media_type == *orig_content_type.media_type()
                            }
                        } else {
                            false
                        }
                    }
                },
                None => false,
            })
            .count()
            > 0
    }
}
