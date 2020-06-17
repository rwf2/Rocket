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
//!         assert_eq!(response.body_string().await, Some("Hello, world!".into()));
//!     }
//! }
//! ```
//!
//! [`Client`]: crate::local::asynchronous::Client
//! [`LocalRequest`]: crate::local::LocalRequest

#[macro_use]
mod client;
pub mod blocking;
pub mod asynchronous;

// FIXME: Where should this documentation go? Perhaps top-level to avoid
// duplication in each `Client`?

/// A structure to construct requests for local dispatching.
///
/// # Usage
///
/// A `Client` is constructed via the [`new()`] or [`untracked()`] methods from
/// an already constructed `Rocket` instance. Once a value of `Client` has been
/// constructed, the [`LocalRequest`] constructor methods ([`get()`], [`put()`],
/// [`post()`], and so on) can be used to create a `LocalRequest` for
/// dispatching.
///
/// See the [top-level documentation](crate::local) for more usage information.
///
/// ## Cookie Tracking
///
/// A `Client` constructed using [`new()`] propagates cookie changes made by
/// responses to previously dispatched requests. In other words, if a previously
/// dispatched request resulted in a response that adds a cookie, any future
/// requests will contain that cookie. Similarly, cookies removed by a response
/// won't be propagated further.
///
/// This is typically the desired mode of operation for a `Client` as it removes
/// the burden of manually tracking cookies. Under some circumstances, however,
/// disabling this tracking may be desired. In these cases, use the
/// [`untracked()`](Client::untracked()) constructor to create a `Client` that
/// _will not_ track cookies.
///
/// ### Synchronization
///
/// While `Client` implements `Sync`, using it in a multithreaded environment
/// while tracking cookies can result in surprising, non-deterministic behavior.
/// This is because while cookie modifications are serialized, the exact
/// ordering depends on when requests are dispatched. Specifically, when cookie
/// tracking is enabled, all request dispatches are serialized, which in-turn
/// serializes modifications to the internally tracked cookies.
///
/// If possible, refrain from sharing a single instance of `Client` across
/// multiple threads. Instead, prefer to create a unique instance of `Client`
/// per thread. If it's not possible, ensure that either you are not depending
/// on cookies, the ordering of their modifications, or both, or have arranged
/// for dispatches to occur in a deterministic ordering.
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local request to `POST /` with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::asynchronous::Client;
///
/// # rocket::async_test(async {
/// let rocket = rocket::ignite();
/// let client = Client::new(rocket).await.expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch().await;
/// # });
/// ```
///
/// [`new()`]: #method.new
/// [`untracked()`]: #method.untracked
/// [`get()`]: #method.get
/// [`put()`]: #method.put
/// [`post()`]: #method.post
fn client() {}
