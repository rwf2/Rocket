//! Structures for local dispatching of requests, primarily for testing.
//!
//! This module allows for simple request dispatching against a local,
//! non-networked instance of Rocket. The primary use of this module is to unit
//! and integration test Rocket applications by crafting requests, dispatching
//! them, and verifying the response.
//!
//! # Usage
//!
//! This module contains a [`Client`] structure that is used to create
//! [`LocalRequest`] structures that can be dispatched against a given
//! [`Rocket`] instance. Usage is straightforward:
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
//!      # use rocket::local::Client;
//!      # let rocket = rocket::ignite();
//!      let client = Client::new(rocket).expect("valid rocket instance");
//!      # let _ = client;
//!      ```
//!
//!   3. Construct requests using the `Client` instance.
//!
//!      ```rust
//!      # use rocket::local::Client;
//!      # let rocket = rocket::ignite();
//!      # let client = Client::new(rocket).unwrap();
//!      let req = client.get("/");
//!      # let _ = req;
//!      ```
//!
//!   3. Dispatch the request to retrieve the response.
//!
//!      ```rust
//!      # use rocket::local::Client;
//!      # let rocket = rocket::ignite();
//!      # let client = Client::new(rocket).unwrap();
//!      # let req = client.get("/");
//!      let response = req.dispatch();
//!      # let _ = response;
//!      ```
//!
//! All together and in idiomatic fashion, this might look like:
//!
//! ```rust
//! use rocket::local::Client;
//!
//! let client = Client::new(rocket::ignite()).expect("valid rocket");
//! let response = client.post("/")
//!     .body("Hello, world!")
//!     .dispatch();
//! # let _ = response;
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
//!     use rocket::local::Client;
//!
//!     #[test]
//!     fn test_hello_world() {
//!         // Construct a client to use for dispatching requests.
//!         let rocket = rocket::ignite().mount("/", routes![hello]);
//!         let client = Client::new(rocket).expect("valid rocket instance");
//!
//!         // Dispatch a request to 'GET /' and validate the response.
//!         let mut response = client.get("/").dispatch();
//!         assert_eq!(response.body_string(), Some("Hello, world!".into()));
//!     }
//! }
//! ```
//!
//! [`Client`]: local::Client
//! [`LocalRequest`]: local::LocalRequest

mod request;
mod client;

pub use self::request::{LocalResponse, LocalRequest};
pub use self::client::Client;
