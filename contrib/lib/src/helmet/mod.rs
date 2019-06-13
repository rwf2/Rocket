//! Security and privacy headers for all outgoing responses.
//!
//! [`SpaceHelmet`] provides a typed interface for HTTP security headers. It
//! takes some inspiration from [helmetjs], a similar piece of middleware for
//! [express].
//!
//! [fairing]: https://rocket.rs/v0.5/guide/fairings/
//! [helmetjs]: https://helmetjs.github.io/
//! [express]: https://expressjs.com
//! [`SpaceHelmet`]: helmet::SpaceHelmet
//!
//! # Enabling
//!
//! This module is only available when the `helmet` feature is enabled. Enable
//! it in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["helmet"]
//! ```
//!
//! # Supported Headers
//!
//! | HTTP Header                 | Description                            | Policy       | Default? |
//! | --------------------------- | -------------------------------------- | ------------ | -------- |
//! | [X-XSS-Protection]          | Prevents some reflected XSS attacks.   | [`XssFilter`]      | ✔        |
//! | [X-Content-Type-Options]    | Prevents client sniffing of MIME type. | [`NoSniff`]  | ✔        |
//! | [X-Frame-Options]           | Prevents [clickjacking].               | [`Frame`]    | ✔        |
//! | [Strict-Transport-Security] | Enforces strict use of HTTPS.          | [`Hsts`]     | ?        |
//! | [Expect-CT]                 | Enables certificate transparency.      | [`ExpectCt`] | ✗        |
//! | [Referrer-Policy]           | Enables referrer policy.               | [`Referrer`] | ✗        |
//!
//! <small>? If TLS is enabled when the application is launched, in a
//! non-development environment (e.g., staging or production), HSTS is
//! automatically enabled with its default policy and a warning is
//! issued.</small>
//!
//! [X-XSS-Protection]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection
//! [X-Content-Type-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options
//! [X-Frame-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options
//! [Strict-Transport-Security]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
//! [Expect-CT]:  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT
//! [Referrer-Policy]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy
//! [clickjacking]: https://en.wikipedia.org/wiki/Clickjacking
//!
//! [`XssFilter`]: helmet::XssFilter
//! [`NoSniff`]: helmet::NoSniff
//! [`Frame`]: helmet::Frame
//! [`Hsts`]: helmet::Hsts
//! [`ExpectCt`]: helmet::ExpectCt
//! [`Referrer`]: helmet::Referrer
//!
//! # Usage
//!
//! To apply default headers, simply attach an instance of [`SpaceHelmet`]
//! before launching:
//!
//! ```rust
//! # extern crate rocket;
//! # extern crate rocket_contrib;
//! use rocket_contrib::helmet::SpaceHelmet;
//!
//! let rocket = rocket::ignite().attach(SpaceHelmet::default());
//! ```
//!
//! Each header can be configured individually. To enable a particular header,
//! call the chainable [`enable()`](helmet::SpaceHelmet::enable()) method
//! on an instance of `SpaceHelmet`, passing in the configured policy type.
//! Similarly, to disable a header, call the chainable
//! [`disable()`](helmet::SpaceHelmet::disable()) method on an instance of
//! `SpaceHelmet`:
//!
//! ```rust
//! # extern crate rocket;
//! # extern crate rocket_contrib;
//! use rocket::http::uri::Uri;
//! use rocket_contrib::helmet::{SpaceHelmet, Frame, XssFilter, Hsts, NoSniff};
//!
//! let site_uri = Uri::parse("https://mysite.example.com").unwrap();
//! let report_uri = Uri::parse("https://report.example.com").unwrap();
//! let helmet = SpaceHelmet::default()
//!     .enable(Hsts::default())
//!     .enable(Frame::AllowFrom(site_uri))
//!     .enable(XssFilter::EnableReport(report_uri))
//!     .disable::<NoSniff>();
//! ```
//!
//! # FAQ
//!
//! * **Which policies should I choose?**
//!
//!   See the links in the table above for individual header documentation. The
//!   [helmetjs] docs are also a good resource, and [OWASP] has a collection of
//!   references on these headers.
//!
//! * **Do I need any headers beyond what `SpaceHelmet` enables by default?**
//!
//!   Maybe! The other headers can protect against many important
//!   vulnerabilities. Please consult their documentation and other resources to
//!   determine if they are needed for your project.
//!
//! [OWASP]: https://www.owasp.org/index.php/OWASP_Secure_Headers_Project#tab=Headers

mod helmet;
mod policy;

pub use self::helmet::SpaceHelmet;
pub use self::policy::*;
