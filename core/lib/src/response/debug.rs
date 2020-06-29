use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::Status;

use yansi::Paint;

/// Debug prints the internal value before responding with a 500 error.
///
/// This value exists primarily to allow handler return types that would not
/// otherwise implement [`Responder`]. It is typically used in conjunction with
/// `Result<T, E>` where `E` implements `Debug` but not `Responder`.
///
/// # Example
///
/// Because of the generic `From<E>` implementation for `Debug<E>`, conversions
/// from `Result<T, E>` to `Result<T, Debug<E>>` through `?` occur
/// automatically:
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// use std::io;
///
/// use tokio::io::AsyncReadExt;
///
/// # use rocket::post;
/// use rocket::Data;
/// use rocket::response::Debug;
///
/// #[post("/", format = "plain", data = "<data>")]
/// async fn post(data: Data) -> Result<String, Debug<io::Error>> {
///     let mut name = String::with_capacity(32);
///     data.open().take(32).read_to_string(&mut name).await?;
///     Ok(name)
/// }
/// ```
///
/// It is also possible to map the error directly to `Debug` via
/// [`Result::map_err()`]:
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// use std::string::FromUtf8Error;
///
/// # use rocket::get;
/// use rocket::response::Debug;
///
/// #[get("/")]
/// fn rand_str() -> Result<String, Debug<FromUtf8Error>> {
///     # /*
///     let bytes: Vec<u8> = random_bytes();
///     # */
///     # let bytes: Vec<u8> = vec![];
///     String::from_utf8(bytes).map_err(Debug)
/// }
/// ```
#[derive(Debug)]
pub struct Debug<E>(pub E);

impl<E> From<E> for Debug<E> {
    #[inline(always)]
    fn from(e: E) -> Self {
        Debug(e)
    }
}

impl<'r, E: std::fmt::Debug> Responder<'r, 'static> for Debug<E> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        warn_!("Debug: {:?}", Paint::default(self.0));
        warn_!("Debug always responds with {}.", Status::InternalServerError);
        Response::build().status(Status::InternalServerError).ok()
    }
}
