//! Security and privacy headers for all outgoing responses.
//!
//! The [`Shield`] fairing provides a typed interface for injecting HTTP
//! security and privacy headers into all outgoing responses. It takes some
//! inspiration from [helmetjs], a similar piece of middleware for [express].
//!
//! [fairing]: https://rocket.rs/v0.5-rc/guide/fairings/
//! [helmetjs]: https://helmetjs.github.io/
//! [express]: https://expressjs.com
//!
//! # Supported Headers
//!
//! | HTTP Header                 | Description                            | Policy         | Default? |
//! | --------------------------- | -------------------------------------- | -------------- | -------- |
//! | [X-XSS-Protection]          | Prevents some reflected XSS attacks.   | [`XssFilter`]  | ✗        |
//! | [X-Content-Type-Options]    | Prevents client sniffing of MIME type. | [`NoSniff`]    | ✔        |
//! | [X-Frame-Options]           | Prevents [clickjacking].               | [`Frame`]      | ✔        |
//! | [Strict-Transport-Security] | Enforces strict use of HTTPS.          | [`Hsts`]       | ?        |
//! | [Expect-CT]                 | Enables certificate transparency.      | [`ExpectCt`]   | ✗        |
//! | [Referrer-Policy]           | Enables referrer policy.               | [`Referrer`]   | ✗        |
//! | [X-DNS-Prefetch-Control]    | Controls browser DNS prefetching.      | [`Prefetch`]   | ✗        |
//! | [Permissions-Policy]        | Allows or block browser features.      | [`Permission`] | ✔        |
//!
//! <small>? If TLS is enabled in a non-debug profile, HSTS is automatically
//! enabled with its default policy and a warning is logged at liftoff.</small>
//!
//! [X-XSS-Protection]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection
//! [X-Content-Type-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options
//! [X-Frame-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options
//! [Strict-Transport-Security]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
//! [Expect-CT]:  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT
//! [Referrer-Policy]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy
//! [X-DNS-Prefetch-Control]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-DNS-Prefetch-Control
//! [clickjacking]: https://en.wikipedia.org/wiki/Clickjacking
//! [Permissions-Policy]: https://github.com/w3c/webappsec-permissions-policy/blob/a45df7b237e2a85e1909d7f226ca4eb4ce5095ba/permissions-policy-explainer.md
//!
//! [`XssFilter`]: self::XssFilter
//! [`NoSniff`]: self::NoSniff
//! [`Frame`]: self::Frame
//! [`Hsts`]: self::Hsts
//! [`ExpectCt`]: self::ExpectCt
//! [`Referrer`]: self::Referrer
//! [`Prefetch`]: self::Prefetch
//!
//! # Usage
//!
//! By default, [`Shield::default()`] is attached to all instances Rocket. To
//! change the default, including removing all `Shield` headers, attach a
//! configured instance of [`Shield`]:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::shield::Shield;
//!
//! #[launch]
//! fn rocket() -> _ {
//!     // Remove all `Shield` headers.
//!     rocket::build().attach(Shield::new())
//! }
//! ```
//!
//! Each header can be configured individually. To enable a particular header,
//! call the chainable [`enable()`](shield::Shield::enable()) method
//! on an instance of `Shield`, passing in the configured policy type.
//! Similarly, to disable a header, call the chainable
//! [`disable()`](shield::Shield::disable()) method on an instance of
//! `Shield`:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use time::Duration;
//!
//! use rocket::http::uri::Uri;
//! use rocket::shield::{Shield, Referrer, Prefetch, ExpectCt, NoSniff};
//!
//! let report_uri = uri!("https://report.rocket.rs");
//! let shield = Shield::default()
//!     .enable(Referrer::NoReferrer)
//!     .enable(Prefetch::Off)
//!     .enable(ExpectCt::ReportAndEnforce(Duration::days(30), report_uri))
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
//! * **Do I need any headers beyond what `Shield` enables by default?**
//!
//!   Maybe! The other headers may protect against many important
//!   vulnerabilities. Please consult their documentation and other resources to
//!   determine if they are needed for your project.
//!
//! [OWASP]: https://www.owasp.org/index.php/OWASP_Secure_Headers_Project#tab=Headers

mod shield;
mod policy;

pub use self::shield::Shield;
pub use self::policy::*;
