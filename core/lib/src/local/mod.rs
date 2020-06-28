//! Structures for local dispatching of requests, primarily for testing.
//!
//! This module allows for simple request dispatching against a local,
//! non-networked instance of Rocket. The primary use of this module is to unit
//! and integration test Rocket applications by crafting requests, dispatching
//! them, and verifying the response.
//!
//! # Usage
//!
//! This module contains two variants of the local API: [`asynchronous`] and
//! [`blocking`]. The primary difference between the two is in usage: the
//! `asynchronous` API requires an asynchronous test entry point such as
//! `#[rocket::async_test]`, while the `blocking` API can be used with
//! `#[test]`. Additionally, several methods in the `asynchronous` API are
//! `async` and must therefore be `await`ed.
//!
//! Both APIs include a [`Client`] structure that is used to create
//! [`LocalRequest`] structures that can be dispatched against a given
//! [`Rocket`](crate::Rocket) instance. Usage is straightforward:
//!
//!   1. Construct a `Rocket` instance that represents the application.
//!
//!      ```rust
//!      let rocket = rocket::ignite();
//!      # let _ = rocket;
//!      ```
//!
//!   2. Construct a `Client` using the `Rocket` instance.
//!
//!      ```rust
//!      # use rocket::local::asynchronous::Client;
//!      # let rocket = rocket::ignite();
//!      # rocket::async_test(async {
//!      let client = Client::new(rocket).await.expect("valid rocket instance");
//!      # let _ = client;
//!      # });
//!      ```
//!
//!   3. Construct requests using the `Client` instance.
//!
//!      ```rust
//!      # use rocket::local::asynchronous::Client;
//!      # let rocket = rocket::ignite();
//!      # rocket::async_test(async {
//!      # let client = Client::new(rocket).await.unwrap();
//!      let req = client.get("/");
//!      # let _ = req;
//!      # });
//!      ```
//!
//!   3. Dispatch the request to retrieve the response.
//!
//!      ```rust
//!      # use rocket::local::asynchronous::Client;
//!      # let rocket = rocket::ignite();
//!      # rocket::async_test(async {
//!      # let client = Client::new(rocket).await.unwrap();
//!      # let req = client.get("/");
//!      let response = req.dispatch().await;
//!      # let _ = response;
//!      # });
//!      ```
//!
//! All together and in idiomatic fashion, this might look like:
//!
//! ```rust
//! use rocket::local::asynchronous::Client;
//!
//! # rocket::async_test(async {
//! let client = Client::new(rocket::ignite()).await.expect("valid rocket");
//! let response = client.post("/")
//!     .body("Hello, world!")
//!     .dispatch().await;
//! # let _ = response;
//! # });
//! ```
//!
//! # Unit/Integration Testing
//!
//! This module can be used to test a Rocket application by constructing
//! requests via `Client` and validating the resulting response. As an example,
//! consider the following complete "Hello, world!" application, with testing.
//!
//! ```rust
//! #![feature(proc_macro_hygiene)]
//!
//! #[macro_use] extern crate rocket;
//!
//! #[get("/")]
//! fn hello() -> &'static str {
//!     "Hello, world!"
//! }
//!
//! # fn main() {  }
//! #[cfg(test)]
//! mod test {
//!     use super::{rocket, hello};
//!     use rocket::local::asynchronous::Client;
//!
//!     #[rocket::async_test]
//!     fn test_hello_world() {
//!         // Construct a client to use for dispatching requests.
//!         let rocket = rocket::ignite().mount("/", routes![hello]);
//!         let client = Client::new(rocket).expect("valid rocket instance");
//!
//!         // Dispatch a request to 'GET /' and validate the response.
//!         let mut response = client.get("/").dispatch().await;
//!         assert_eq!(response.into_string().await, Some("Hello, world!".into()));
//!     }
//! }
//! ```
//!
//! [`Client`]: crate::local::asynchronous::Client
//! [`LocalRequest`]: crate::local::asynchronous::LocalRequest

#[macro_use] mod client;
#[macro_use] mod request;
#[macro_use] mod response;

pub mod asynchronous;
pub mod blocking;
