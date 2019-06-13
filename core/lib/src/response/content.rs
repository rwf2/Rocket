//! Contains types that set the Content-Type of a response.
//!
//! # Usage
//!
//! Each type wraps a given responder. The `Responder` implementation of each
//! type replaces the Content-Type of the wrapped responder and delegates the
//! remainder of the response to the wrapped responder. This allows for setting
//! the Content-Type of a type that doesn't set it itself or for overriding one
//! that does.
//!
//! # Example
//!
//! The following snippet creates an `Html` content response for a string.
//! Normally, raw strings set their response Content-Type to `text/plain`. By
//! using the `Html` content response, the Content-Type will be set to
//! `text/html` instead.
//!
//! ```rust
//! use rocket::response::content;
//!
//! # #[allow(unused_variables)]
//! let response = content::Html("<h1>Hello, world!</h1>");
//! ```

use crate::request::Request;
use crate::response::{Response, Responder};
use crate::http::{Status, ContentType};

/// Sets the Content-Type of a `Responder` to a chosen value.
///
/// Delegates the remainder of the response to the wrapped responder.
///
/// # Example
///
/// Set the Content-Type of a string to PDF.
///
/// ```rust
/// use rocket::response::content::Content;
/// use rocket::http::ContentType;
///
/// # #[allow(unused_variables)]
/// let response = Content(ContentType::PDF, "Hi.");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Content<R>(pub ContentType, pub R);

/// Overrides the Content-Type of the response to the wrapped `ContentType` then
/// delegates the remainder of the response to the wrapped responder.
impl<'r, R: Responder<'r>> Responder<'r> for Content<R> {
    #[inline(always)]
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        Response::build()
            .merge(self.1.respond_to(req)?)
            .header(self.0)
            .ok()
    }
}

macro_rules! ctrs {
    ($($name:ident: $ct:ident, $name_str:expr, $ct_str:expr),+) => {
        $(
            #[doc="Override the `Content-Type` of the response to <b>"]
            #[doc=$name_str]
            #[doc="</b>, or <i>"]
            #[doc=$ct_str]
            #[doc="</i>."]
            ///
            /// Delegates the remainder of the response to the wrapped responder.
            #[derive(Debug, Clone, PartialEq)]
            pub struct $name<R>(pub R);

            /// Sets the Content-Type of the response then delegates the
            /// remainder of the response to the wrapped responder.
            impl<'r, R: Responder<'r>> Responder<'r> for $name<R> {
                fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
                    Content(ContentType::$ct, self.0).respond_to(req)
                }
            }
        )+
    }
}

ctrs! {
    // FIXME: Add a note that this is _not_ the `contrib` `Json`.
    Json: JSON, "JSON", "application/json",
    Xml: XML, "XML", "text/xml",
    MsgPack: MsgPack, "MessagePack", "application/msgpack",
    Html: HTML, "HTML", "text/html",
    Plain: Plain, "plain text", "text/plain",
    Css: CSS, "CSS", "text/css",
    JavaScript: JavaScript, "JavaScript", "application/javascript"
}
