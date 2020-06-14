use std::fs::File;
use std::io::Cursor;

use tokio::io::BufReader;
use futures::future::BoxFuture;

use crate::http::{Status, ContentType, StatusClass};
use crate::response::{self, Response, Body};
use crate::request::Request;

/// Trait implemented by types that generate responses for clients.
///
/// Types that implement this trait can be used as the return type of a handler,
/// as illustrated below with `T`:
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// # #[macro_use] extern crate rocket;
/// # type T = ();
/// #
/// #[get("/")]
/// fn index() -> T { /* ... */ }
/// ```
///
/// In this example, `T` can be any type, as long as it implements `Responder`.
///
/// # Return Value
///
/// A `Responder` returns a `Future` whose output type is an `Ok(Response)` or
/// an `Err(Status)`:
///
///   * An `Ok` variant means that the `Responder` was successful in generating
///     a `Response`. The `Response` will be written out to the client.
///
///   * An `Err` variant means that the `Responder` could not or did not
///     generate a `Response`. The contained `Status` will be used to find the
///     relevant error catcher which then generates an error response.
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
///     on the file's extension, use [`NamedFile`](crate::response::NamedFile).
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
/// # Implementation Tips
///
/// This section describes a few best practices to take into account when
/// implementing `Responder`.
///
/// ## Joining and Merging
///
/// When chaining/wrapping other `Responder`s, use the
/// [`merge()`](Response::merge()) or [`join()`](Response::join()) methods on
/// the `Response` or `ResponseBuilder` struct. Ensure that you document the
/// merging or joining behavior appropriately.
///
/// ## Inspecting Requests
///
/// A `Responder` has access to the request it is responding to. Even so, you
/// should avoid using the `Request` value as much as possible. This is because
/// using the `Request` object makes your responder _impure_, and so the use of
/// the type as a `Responder` has less intrinsic meaning associated with it. If
/// the `Responder` were pure, however, it would always respond in the same manner,
/// regardless of the incoming request. Thus, knowing the type is sufficient to
/// fully determine its functionality.
///
/// # Async Trait
///
/// [`Responder`] is an _async_ trait. Implementations of `Responder` may be
/// decorated with an attribute of `#[rocket::async_trait]`, allowing the
/// `respond_to` method to be implemented as an `async fn`:
///
/// ```rust
/// use rocket::response::{self, Responder};
/// use rocket::request::Request;
/// # struct MyType;
///
/// #[rocket::async_trait]
/// impl<'r> Responder<'r> for MyType {
///     async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
///         /* .. */
///         # unimplemented!()
///     }
/// }
/// ```
///
/// In certain cases, including when implementing a _wrapping_ responder, it may
/// be desirable to perform work before entering an `async` section, or to
/// return a future directly. In this case, `Responder` can be implemented as:
///
/// ```rust
/// use rocket::response::{self, Responder};
/// use rocket::futures::future::BoxFuture;
/// use rocket::request::Request;
/// # struct MyType<R> { inner: R };
///
/// impl<'r, R: Responder<'r>> Responder<'r> for MyType<R> {
///     fn respond_to<'a, 'x>(self, req: &'r Request<'a>) -> BoxFuture<'x, response::Result<'r>>
///         where 'a: 'x, 'r: 'x, Self: 'x
///     {
///         self.inner.respond_to(req)
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
/// # #![feature(proc_macro_hygiene)]
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
/// #[rocket::async_trait]
/// impl<'r> Responder<'r> for Person {
///     async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
///         let person_string = format!("{}:{}", self.name, self.age);
///         Response::build()
///             .sized_body(Cursor::new(person_string)).await
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
#[crate::async_trait]
pub trait Responder<'r> {
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
    async fn respond_to(self, request: &'r Request<'_>) -> response::Result<'r>;
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for &'r str {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self)).await
            .ok()
    }
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for String {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self)).await
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for &'r [u8] {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(Cursor::new(self)).await
            .ok()
    }
}

/// Returns a response with Content-Type `application/octet-stream` and a
/// fixed-size body containing the data in `self`. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for Vec<u8> {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(Cursor::new(self)).await
            .ok()
    }
}

/// Returns a response with a sized body for the file. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for File {
    async fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        tokio::fs::File::from(self).respond_to(req).await
    }
}

/// Returns a response with a sized body for the file. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for tokio::fs::File {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        let metadata = self.metadata().await;
        let stream = BufReader::new(self);
        match metadata {
            Ok(md) => Response::build().raw_body(Body::Sized(stream, md.len())).ok(),
            Err(_) => Response::build().streamed_body(stream).ok()
        }
    }
}

/// Returns an empty, default `Response`. Always returns `Ok`.
#[crate::async_trait]
impl<'r> Responder<'r> for () {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Ok(Response::new())
    }
}

/// If `self` is `Some`, responds with the wrapped `Responder`. Otherwise prints
/// a warning message and returns an `Err` of `Status::NotFound`.
impl<'r, R: Responder<'r>> Responder<'r> for Option<R> {
    fn respond_to<'a, 'x>(self, req: &'r Request<'a>) -> BoxFuture<'x, response::Result<'r>>
        where 'a: 'x, 'r: 'x, Self: 'x
    {
        match self {
            Some(r) => r.respond_to(req),
            None => {
                warn_!("Response was `None`.");
                Box::pin(async { Err(Status::NotFound) })
            },
        }
    }
}

// Responds with the wrapped `Responder` in `self`, whether it is `Ok` or
/// `Err`.
impl<'r, R: Responder<'r>, E: Responder<'r>> Responder<'r> for Result<R, E> {
    fn respond_to<'a, 'x>(self, req: &'r Request<'a>) -> BoxFuture<'x, response::Result<'r>>
        where 'a: 'x, 'r: 'x, Self: 'x
    {
        match self {
            Ok(responder) => responder.respond_to(req),
            Err(responder) => responder.respond_to(req),
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
#[crate::async_trait]
impl<'r> Responder<'r> for Status {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
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
                warn_!("Fowarding to 500 (Internal Server Error) catcher.");
                Err(Status::InternalServerError)
            }
        }
    }
}
