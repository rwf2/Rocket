use std::fs::File;
use std::io::Cursor;
use std::fmt;

use http::{Status, ContentType};
use response::{Response, Stream};

/// Trait implemented by types that generate responses for clients.
///
/// Types that implement this trait can be used as the return type of a handler,
/// as illustrated below:
///
/// ```rust,ignore
/// #[get("/")]
/// fn index() -> T { ... }
/// ```
///
/// In this example, `T` can be any type that implements `Responder`.
///
/// # Return Value
///
/// A `Responder` returns an `Ok(Response)` or an `Err(Status)`:
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
///     Sets the `Content-Type`t to `text/plain`. The string is used as the body
///     of the response, which is fixed size and not streamed. To stream a
///     string, use `Stream::from(Cursor::new(string))`.
///
///   * **File**
///
///     Streams the `File` to the client. This is essentially an alias to
///     `Stream::from(file)`.
///
///   * **()**
///
///     Responds with an empty body. No Content-Type is set.
///
///   * **Option&lt;T>**
///
///     If the `Option` is `Some`, the wrapped responder is used to respond to
///     the client. Otherwise, an `Err` with status **404 Not Found** is
///     returned and a warning is printed to the console.
///
///   * **Result&lt;T, E>** _where_ **E: Debug**
///
///     If the `Result` is `Ok`, the wrapped responder is used to respond to the
///     client. Otherwise, an `Err` with status **500 Internal Server Error** is
///     returned and the error is printed to the console using the `Debug`
///     implementation.
///
///   * **Result&lt;T, E>** _where_ **E: Debug + Responder**
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
/// ## Debug
///
/// A type implementing `Responder` should implement the `Debug` trait when
/// possible. This is because the `Responder` implementation for `Result`
/// requires its `Err` type to implement `Debug`. Therefore, a type implementing
/// `Debug` can more easily be composed.
///
/// ## Joining and Merging
///
/// When chaining/wrapping other `Responder`s, use the
/// [merge](/rocket/struct.Response.html#method.merge) or
/// [join](/rocket/struct.Response.html#method.join) methods on the `Response`
/// or `ResponseBuilder` struct. Ensure that you document the merging or joining
/// behavior appropriately.
///
/// # Example
///
/// Say that you have a custom type, `Person`:
///
/// ```rust
///
/// # #[allow(dead_code)]
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// You'd like to use `Person` as a `Responder` so that you can return a
/// `Person` directly from a handler:
///
/// ```rust,ignore
/// #[get("/person/<id>")]
/// fn person(id: usize) -> Option<Person> {
///     Person::from_id(id)
/// }
/// ```
///
/// You want the `Person` responder to set two header fields: `X-Person-Name`
/// and `X-Person-Age` as well as supply a custom representation of the object
/// (`Content-Type: application/x-person`) in the body of the response. The
/// following `Responder` implementation accomplishes this:
///
/// ```rust
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #
/// # #[derive(Debug)]
/// # struct Person { name: String, age: u16 }
/// #
/// use std::io::Cursor;
///
/// use rocket::response::{self, Response, Responder};
/// use rocket::http::ContentType;
///
/// impl<'r> Responder<'r> for Person {
///     fn respond(self) -> response::Result<'r> {
///         Response::build()
///             .sized_body(Cursor::new(format!("{}:{}", self.name, self.age)))
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
pub trait Responder<'r> {
    /// Returns `Ok` if a `Response` could be generated successfully. Otherwise,
    /// returns an `Err` with a failing `Status`.
    ///
    /// When using Rocket's code generation, if an `Ok(Response)` is returned,
    /// the response will be written out to the client. If an `Err(Status)` is
    /// returned, the error catcher for the given status is retrieved and called
    /// to generate a final error response, which is then written out to the
    /// client.
    fn respond(self) -> Result<Response<'r>, Status>;
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
///
/// # Example
///
/// ```rust
/// use rocket::response::Responder;
/// use rocket::http::ContentType;
///
/// let mut response = "Hello".respond().unwrap();
/// assert_eq!(response.body_string(), Some("Hello".into()));
///
/// let content_type: Vec<_> = response.headers().get("Content-Type").collect();
/// assert_eq!(content_type.len(), 1);
/// assert_eq!(content_type[0], ContentType::Plain.to_string());
/// ```
impl<'r> Responder<'r> for &'r str {
    fn respond(self) -> Result<Response<'r>, Status> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self))
            .ok()
    }
}

/// Returns a response with Content-Type `text/plain` and a fixed-size body
/// containing the string `self`. Always returns `Ok`.
impl Responder<'static> for String {
    fn respond(self) -> Result<Response<'static>, Status> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self))
            .ok()
    }
}

/// Aliases Stream<File>: `Stream::from(self)`.
impl Responder<'static> for File {
    fn respond(self) -> Result<Response<'static>, Status> {
        Stream::from(self).respond()
    }
}

/// Returns an empty, default `Response`. Always returns `Ok`.
impl Responder<'static> for () {
    fn respond(self) -> Result<Response<'static>, Status> {
        Ok(Response::new())
    }
}

/// If `self` is `Some`, responds with the wrapped `Responder`. Otherwise prints
/// a warning message and returns an `Err` of `Status::NotFound`.
impl<'r, R: Responder<'r>> Responder<'r> for Option<R> {
    fn respond(self) -> Result<Response<'r>, Status> {
        self.map_or_else(|| {
            warn_!("Response was `None`.");
            Err(Status::NotFound)
        }, |r| r.respond())
    }
}

/// If `self` is `Ok`, responds with the wrapped `Responder`. Otherwise prints
/// an error message with the `Err` value returns an `Err` of
/// `Status::InternalServerError`.
impl<'r, R: Responder<'r>, E: fmt::Debug> Responder<'r> for Result<R, E> {
    default fn respond(self) -> Result<Response<'r>, Status> {
        self.map(|r| r.respond()).unwrap_or_else(|e| {
            error_!("Response was `Err`: {:?}.", e);
            Err(Status::InternalServerError)
        })
    }
}

/// Responds with the wrapped `Responder` in `self`, whether it is `Ok` or
/// `Err`.
impl<'r, R: Responder<'r>, E: Responder<'r> + fmt::Debug> Responder<'r> for Result<R, E> {
    fn respond(self) -> Result<Response<'r>, Status> {
        match self {
            Ok(responder) => responder.respond(),
            Err(responder) => responder.respond(),
        }
    }
}
