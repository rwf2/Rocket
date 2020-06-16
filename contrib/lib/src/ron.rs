//! Automatic RON (de)serialization support.
//!
//! See the [`Ron`](crate::ron:Ron) type for further details.
//!
//! # Enabling
//!
//! This module is only available when the `ron` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["ron"]
//! ```

use std::ops::{Deref, DerefMut};
use std::io;

use crate::rocket::tokio::io::AsyncReadExt;
use rocket::futures::future::BoxFuture;

use rocket::request::Request;
use rocket::outcome::Outcome::*;
use rocket::data::{Transform::*, Transformed, Data, FromData, TransformFuture, FromDataFuture};
use rocket::response::{self, Responder, content};
use rocket::http::Status;

use serde::Serialize;
use serde::de::Deserialize;

/// The RON type: implements [`FromData`] and [`Responder`], allowing you to
/// easily consume and respond with RON.
///
/// ## Receiving RON
///
/// If you're receiving RON data, simply add a `data` parameter to your route
/// arguments and ensure the type of the parameter is a `Ron<T>`, where `T` is
/// some type you'd like to parse from RON. `T` must implement [`Deserialize`]
/// or from [`serde`]. The data is parsed from the HTTP request body.
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// # #[macro_use] extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// use rocket_contrib::ron::Ron;
///
/// #[post("/users", data = "<user>")]
/// fn new_user(user: Ron<User>) {
///     /* ... */
/// }
/// ```
///
/// ## Sending RON
///
/// If you're responding with RON data, return a `Ron<T>` type, where `T`
/// implements [`Serialize`] from [`serde`]. The content type of the response is
/// set to `text/plain` automatically.
///
/// ```rust
/// # #![feature(proc_macro_hygiene)]
/// # #[macro_use] extern crate rocket;
/// # extern crate rocket_contrib;
/// # type User = usize;
/// use rocket_contrib::ron::Ron;
///
/// #[get("/users/<id>")]
/// fn user(id: usize) -> Ron<User> {
///     let user_from_id = User::from(id);
///     /* ... */
///     Ron(user_from_id)
/// }
/// ```
///
/// ## Incoming Data Limits
///
/// The default size limit for incoming RON data is 1MiB. Setting a limit
/// protects your application from denial of service (DoS) attacks and from
/// resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.ron` configuration parameter. For
/// instance, to increase the RON limit to 5MiB for all environments, you may
/// add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// ron = 5242880
/// ```
#[derive(Debug)]
pub struct Ron<T>(pub T);

impl<T> Ron<T> {
    /// Consumes the RON wrapper and returns the wrapped item.
    ///
    /// # Example
    /// ```rust
    /// # use rocket_contrib::ron::Ron;
    /// let string = "Hello".to_string();
    /// let my_ron = Ron(string);
    /// assert_eq!(my_ron.into_inner(), "Hello".to_string());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}


/// Default limit for RON is 1MB.
const LIMIT: u64 = 1 << 20;

/// An error returned by the [`Ron`] data guard when incoming data fails to
/// serialize as RON.
#[derive(Debug)]
pub enum RonError<'a> {
    /// An I/O error occurred while reading the incoming request data.
    Io(io::Error),

    /// The client's data was received successfully but failed to parse as valid
    /// RON or as the requested type. The `&str` value in `.0` is the raw data
    /// received from the user, while the `Error` in `.1` is the deserialization
    /// error from `serde`.
    Parse(&'a str, ron_crate::de::Error),
}

impl<'a, T: Deserialize<'a>> FromData<'a> for Ron<T> {
    type Error = RonError<'a>;
    type Owned = String;
    type Borrowed= str;
 
    fn transform<'r>(r: &'r Request<'_>, d: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        let size_limit = r.limits().get("ron").unwrap_or(LIMIT);
        Box::pin(async move {
            let mut s = String::with_capacity(512);
            let mut reader = d.open().take(size_limit);
            match reader.read_to_string(&mut s).await {
                Ok(_) => Borrowed(Success(s)),
                Err(e) => Borrowed(Failure((Status::BadRequest, RonError::Io(e))))
            }
        })
    }

    fn from_data(_: &Request<'_>, o: Transformed<'a, Self>) -> FromDataFuture<'a, Self, Self::Error> {
        Box::pin(async move {
            let string = try_outcome!(o.borrowed());
            match ron_crate::de::from_str(&string) {
                Ok(v) => Success(Ron(v)),
                Err(e) => {
                    error_!("Couldn't parse RON body: {:?}", e);
                    Failure((Status::BadRequest, RonError::Parse(string, e)))
                }
            }
        })
    }
}


/// Serializes the wrapped value into RON. Returns a response with Content-Type
/// Text and a fixed-size body with the serialized value. If serialization
/// fails, an `Err` of `Status::InternalServerError` is returned.
impl<'r, T: Serialize> Responder<'r> for Ron<T> {
    fn respond_to<'a, 'x>(self, req: &'r Request<'a>) -> BoxFuture<'x, response::Result<'r>>
        where 'a: 'x, 'r: 'x, Self: 'x
    {
        match ron_crate::ser::to_string_pretty(&self.0, ron_crate::ser::PrettyConfig::default()) {
            Ok(string) => Box::pin(async move { Ok(content::Plain(string).respond_to(req).await.unwrap()) }),
            Err(e) => Box::pin (async move {
                error_!("RON failed to serialize: {:?}", e);
                Err(Status::InternalServerError)
            })
        }
    }
}

impl<T> Deref for Ron<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Ron<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
