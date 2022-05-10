use std::fs::File;
use std::io::Cursor;
use std::sync::Arc;

use crate::http::{Status, ContentType, StatusClass};
use crate::response::{self, Response};
use crate::request::Request;

/// Trait implemented by types that generate responses for clients.
///
/// Any type that implements `Responder` can be used as the return type of a
/// handler:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type T = ();
/// #
/// // This works for any `T` that implements `Responder`.
/// #[get("/")]
/// fn index() -> T { /* ... */ }
/// ```
///
/// # Deriving
///
/// This trait can, and largely _should_, be automatically derived. The derive
/// can handle all simple cases and most complex cases, too. When deriving
/// `Responder`, the first field of the annotated structure (or of each variant
/// if an `enum`) is used to generate a response while the remaining fields are
/// used as response headers:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[cfg(feature = "json")] mod _main {
/// # type Template = String;
/// use rocket::http::ContentType;
/// use rocket::serde::{Serialize, json::Json};
///
/// #[derive(Responder)]
/// enum Error<T> {
///     #[response(status = 400)]
///     Unauthorized(Json<T>),
///     #[response(status = 404)]
///     NotFound(Template, ContentType),
/// }
/// # }
/// ```
///
/// For full details on deriving `Responder`, see the [`Responder` derive].
///
/// [`Responder` derive]: derive@crate::Responder
///
/// # Provided Implementations
///
/// Rocket implements `Responder` for several standard library types. Their
/// behavior is documented here. Note that the `Result` implementation is
/// overloaded, allowing for two `Responder`s to be used at once, depending on
/// the variant.
///
///   * **&str**
///
///     Sets the `Content-Type` to `text/plain`. The string is used as the body
///     of the response, which is fixed size and not streamed. To stream a raw
///     string, use `Stream::from(Cursor::new(string))`.
///
///   * **String**
///
///     Sets the `Content-Type` to `text/plain`. The string is used as the body
///     of the response, which is fixed size and not streamed. To stream a
///     string, use `Stream::from(Cursor::new(string))`.
///
///   * **&\[u8\]**
///
///     Sets the `Content-Type` to `application/octet-stream`. The slice
///     is used as the body of the response, which is fixed size and not
///     streamed. To stream a slice of bytes, use
///     `Stream::from(Cursor::new(data))`.
///
///   * **Vec&lt;u8>**
///
///     Sets the `Content-Type` to `application/octet-stream`. The vector's data
///     is used as the body of the response, which is fixed size and not
///     streamed. To stream a vector of bytes, use
///     `Stream::from(Cursor::new(vec))`.
///
///   * **File**
///
///     Responds with a streamed body containing the data in the `File`. No
///     `Content-Type` is set. To automatically have a `Content-Type` set based
///     on the file's extension, use [`NamedFile`](crate::fs::NamedFile).
///
///   * **()**
///
///     Responds with an empty body. No `Content-Type` is set.
///
///   * **Option&lt;T>**
///
///     If the `Option` is `Some`, the wrapped responder is used to respond to
///     the client. Otherwise, an `Err` with status **404 Not Found** is
///     returned and a warning is printed to the console.
///
///   * **Result&lt;T, E>**
///
///     If the `Result` is `Ok`, the wrapped `Ok` responder is used to respond
///     to the client. If the `Result` is `Err`, the wrapped `Err` responder is
///     used to respond to the client.
///
/// # Return Value
///
/// A `Responder` returns a `Future` whose output type is a `Result<Response,
/// Status>`.
///
///   * An `Ok(Response)` indicates success. The `Response` will be written out
///     to the client.
///
///   * An `Err(Status)` indicates failure. The error catcher for `Status` will
///     be invoked to generate a response.
///
/// # Implementation Tips
///
/// This section describes a few best practices to take into account when
/// implementing `Responder`.
///
/// 1. Avoid Manual Implementations
///
///    The [`Responder` derive] is a powerful mechanism that eliminates the need
///    to implement `Responder` in almost all cases. We encourage you to explore
///    using the derive _before_ attempting to implement `Responder` directly.
///    It allows you to leverage existing `Responder` implementations through
///    composition, decreasing the opportunity for mistakes or performance
///    degradation.
///
/// 2. Joining and Merging
///
///    When chaining/wrapping other `Responder`s, start with
///    [`Response::build_from()`] and/or use the [`merge()`](Response::merge())
///    or [`join()`](Response::join()) methods on the `Response` or
///    `ResponseBuilder` struct. Ensure that you document merging or joining
///    behavior appropriatse.
///
/// 3. Inspecting Requests
///
///    While tempting, a `Responder` that varies its functionality based on the
///    incoming request sacrifices its functionality being understood based
///    purely on its type. By implication, gleaming the functionality of a
///    _handler_ from its type signature also becomes more difficult. You should
///    avoid varying responses based on the `Request` value as much as possible.
///
/// ## Lifetimes
///
/// `Responder` has two lifetimes: `Responder<'r, 'o: 'r>`.
///
///   * `'r` bounds the reference to the `&'r Request`.
///
///   * `'o` bounds the returned `Response<'o>` to values that live at least as
///     long as the request.
///
///     This includes borrows from the `Request` itself (where `'o` would be
///     `'r` as in `impl<'r> Responder<'r, 'r>`) as well as `'static` data
///     (where `'o` would be `'static` as in `impl<'r> Responder<'r, 'static>`).
///
/// In practice, you are likely choosing between four signatures:
///
/// ```rust
/// # use rocket::request::Request;
/// # use rocket::response::{self, Responder};
/// # struct A;
/// // If the response contains no borrowed data.
/// impl<'r> Responder<'r, 'static> for A {
///     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
///         todo!()
///     }
/// }
///
/// # struct B<'r>(&'r str);
/// // If the response borrows from the request.
/// impl<'r> Responder<'r, 'r> for B<'r> {
///     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
///         todo!()
///     }
/// }
///
/// # struct C;
/// // If the response is or wraps a borrow that may outlive the request.
/// impl<'r, 'o: 'r> Responder<'r, 'o> for &'o C {
///     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
///         todo!()
///     }
/// }
///
/// # struct D<R>(R);
/// // If the response wraps an existing responder.
/// impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for D<R> {
///     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
///         todo!()
///     }
/// }
/// ```
///
/// # Example
///
/// Say that you have a custom type, `Person`:
///
/// ```rust
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// You'd like to use `Person` as a `Responder` so that you can return a
/// `Person` directly from a handler:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type Person = String;
/// #[get("/person/<id>")]
/// fn person(id: usize) -> Option<Person> {
///     # /*
///     Person::from_id(id)
///     # */ None
/// }
/// # fn main() {}
/// ```
///
/// You want the `Person` responder to set two header fields: `X-Person-Name`
/// and `X-Person-Age` as well as supply a custom representation of the object
/// (`Content-Type: application/x-person`) in the body of the response. The
/// following `Responder` implementation accomplishes this:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person { name: String, age: u16 }
/// #
/// use std::io::Cursor;
///
/// use rocket::request::Request;
/// use rocket::response::{self, Response, Responder};
/// use rocket::http::ContentType;
///
/// impl<'r> Responder<'r, 'static> for Person {
///     fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
///         let string = format!("{}:{}", self.name, self.age);
///         Response::build_from(string.respond_to(req)?)
///             .raw_header("X-Person-Name", self.name)
///             .raw_header("X-Person-Age", self.age.to_string())
///             .header(ContentType::new("application", "x-person"))
///             .ok()
///     }
/// }
/// #
/// # #[get("/person")]
/// # fn person() -> Person { Person { name: "a".to_string(), age: 20 } }
/// # fn main() {  }
/// ```
///
/// Note that the implementation could have instead been derived if structured
/// in a slightly different manner:
///
/// ```rust
/// use rocket::http::Header;
/// use rocket::response::Responder;
///
/// #[derive(Responder)]
/// #[response(content_type = "application/x-person")]
/// struct Person {
///     text: String,
///     name: Header<'static>,
///     age: Header<'static>,
/// }
///
/// impl Person {
///     fn new(name: &str, age: usize) -> Person {
///         Person {
///             text: format!("{}:{}", name, age),
///             name: Header::new("X-Person-Name", name.to_string()),
///             age: Header::new("X-Person-Age", age.to_string())
///         }
///     }
/// }
/// #
/// # #[rocket::get("/person")]
/// # fn person() -> Person { Person::new("Bob", 29) }
/// ```
pub trait Responder<'r, 'o: 'r> {
    /// Returns `Ok` if a `Response` could be generated successfully. Otherwise,
    /// returns an `Err` with a failing `Status`.
    ///
    /// The `request` parameter is the `Request` that this `Responder` is
    /// responding to.
    ///
    /// When using Rocket's code generation, if an `Ok(Response)` is returned,
    /// the response will be written out to the client. If an `Err(Status)` is
    /// returned, the error catcher for the given status is retrieved and called
    /// to generate a final error response, which is then written out to the
    /// client.
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o>;
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
impl<'r, 'o: 'r> Responder<'r, 'o> for &'o str {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for String {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

#[repr(transparent)]
struct DerefRef<T>(T);

impl<T: std::ops::Deref> AsRef<[u8]> for DerefRef<T> where T::Target: AsRef<[u8]> {
    fn as_ref(&self) -> &[u8] {
        self.0.deref().as_ref()
    }
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for Arc<str> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(self.len(), Cursor::new(DerefRef(self)))
            .ok()
    }
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for Box<str> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(self.len(), Cursor::new(DerefRef(self)))
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
impl<'r, 'o: 'r> Responder<'r, 'o> for &'o [u8] {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for Vec<u8> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for Arc<[u8]> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for Box<[u8]> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(self.len(), Cursor::new(self))
            .ok()
    }
}

/// Returns the response generated by the inner `T`. Note that this explicitly
/// does not support `Box<dyn Responder>`.
///
/// ```rust,compile_fail
/// #[macro_use] extern crate rocket;
///
/// use rocket::response::Responder;
///
/// #[get("/")]
/// fn f() -> Box<dyn Responder<'static,'static>> {
///     Box::new(())
/// }
/// ```
///
/// However, this `impl` allows boxing sized responders:
///
/// ```rust
/// #[macro_use] extern crate rocket;
///
/// #[derive(Responder)]
/// enum Content {
///     Redirect(Box<rocket::response::Redirect>),
///     Text(String),
/// }
///
/// #[get("/")]
/// fn f() -> Option<Box<String>> {
///     None
/// }
///
/// #[get("/")]
/// fn g() -> Content {
///     Content::Text("hello".to_string())
/// }
/// ```
impl<'r, 'o: 'r, T: Responder<'r, 'o> + Sized> Responder<'r, 'o> for Box<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let inner = *self;
        inner.respond_to(req)
    }
}

/// Returns a response with a sized body for the file. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for File {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        tokio::fs::File::from(self).respond_to(req)
    }
}

/// Returns a response with a sized body for the file. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for tokio::fs::File {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build().sized_body(None, self).ok()
    }
}

/// Returns an empty, default `Response`. Always returns `Ok`.
impl<'r> Responder<'r, 'static> for () {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Ok(Response::new())
    }
}

/// Responds with the inner `Responder` in `Cow`.
impl<'r, 'o: 'r, R: ?Sized + ToOwned> Responder<'r, 'o> for std::borrow::Cow<'o, R>
    where &'o R: Responder<'r, 'o> + 'o, <R as ToOwned>::Owned: Responder<'r, 'o> + 'r
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        match self {
            std::borrow::Cow::Borrowed(b) => b.respond_to(req),
            std::borrow::Cow::Owned(o) => o.respond_to(req),
        }
    }
}

/// If `self` is `Some`, responds with the wrapped `Responder`. Otherwise prints
/// a warning message and returns an `Err` of `Status::NotFound`.
impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for Option<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        match self {
            Some(r) => r.respond_to(req),
            None => {
                warn_!("Response was `None`.");
                Err(Status::NotFound)
            },
        }
    }
}

/// Responds with the wrapped `Responder` in `self`, whether it is `Ok` or
/// `Err`.
impl<'r, 'o: 'r, 't: 'o, 'e: 'o, T, E> Responder<'r, 'o> for Result<T, E>
    where T: Responder<'r, 't>, E: Responder<'r, 'e>
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        match self {
            Ok(responder) => responder.respond_to(req),
            Err(responder) => responder.respond_to(req),
        }
    }
}

/// Responds with the wrapped `Responder` in `self`, whether it is `Left` or
/// `Right`.
impl<'r, 'o: 'r, 't: 'o, 'e: 'o, T, E> Responder<'r, 'o> for crate::Either<T, E>
    where T: Responder<'r, 't>, E: Responder<'r, 'e>
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        match self {
            crate::Either::Left(r) => r.respond_to(req),
            crate::Either::Right(r) => r.respond_to(req),
        }
    }
}

/// The response generated by `Status` depends on the status code itself. The
/// table below summarizes the functionality:
///
/// | Status Code Range | Response                              |
/// |-------------------|---------------------------------------|
/// | [400, 599]        | Forwards to catcher for given status. |
/// | 100, [200, 205]   | Empty with status of `self`.          |
/// | All others.       | Invalid. Errors to `500` catcher.     |
///
/// In short, a client or server error status codes will forward to the
/// corresponding error catcher, a successful status code less than `206` or
/// `100` responds with any empty body and the given status code, and all other
/// status code emit an error message and forward to the `500` (internal server
/// error) catcher.
impl<'r> Responder<'r, 'static> for Status {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        match self.class() {
            StatusClass::ClientError | StatusClass::ServerError => Err(self),
            StatusClass::Success if self.code < 206 => {
                Response::build().status(self).ok()
            }
            StatusClass::Informational if self.code == 100 => {
                Response::build().status(self).ok()
            }
            _ => {
                error_!("Invalid status used as responder: {}.", self);
                Err(Status::InternalServerError)
            }
        }
    }
}
