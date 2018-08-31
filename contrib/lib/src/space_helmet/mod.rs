// #![cfg_attr(test, feature(plugin, decl_macro))]
// #![cfg_attr(test, plugin(rocket_codegen))]

//!Space Helmet is a [fairing](https://rocket.rs/guide/fairings/) that turns on browsers security features via. HTTP headers, it provides:
//!
//!* a typed interface to help prevent configuration errors at compile time.
//!
//!* sane defaults as a starting point for many applications.
//!
//!
//!It takes some inspiration from [helmet](https://helmetjs.github.io/), a similar piece of middleware for [express](https://expressjs.com).
//!
//!#### What does it support?
//!
//!|  | HTTP Header| Enabled by Default?|
//!--- | --- | ---
//!| xss_protect - prevents some xss attacks.  | [X-XSS-Protection](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection)|    ✔    |
//!| no_sniff - prevents sniffing of mime type.| [X-Content-Type-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options)    |    ✔    |
//!| frameguard - prevents clickjacking.        | [X-Frame-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options)|    ✔    |
//!| hsts  - enforces strict use of https.      | [Strict-Transport-Security](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security)|    ❓  |
//!| expect_ct - enables use of certificate transparency.      | [Expect-CT](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT)|    ✗    |
//!| referrer_policy - enables use or referrer policy (for privacy).      | [Referrer-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy) | ✗  |
//!
//!❓<i> if tls is enabled for the rocket a Helmet is attached to, hsts will force its use with the default policy and issue a warning.  </i>
//!
//!
//!
//!#### How about a couple examples
//!Adding space helmet to an exisiting application is easy, just create and attach before launch.
//!```rust,ignore
//!use space_helmet::{Helmet};
//!...
//!let helmet = Helmet::new();
//!let rocket = rocket::ignite().mount("/", routes![hello]).attach(helmet).launch();
//!```
//!
//!
//!Every header can be configured individually if desired.
//!```rust,ignore
//!// Every header has a corresponding policy type.
//!use rocket::http::uri::Uri;
//!use space_helmet::{FramePolicy, XSSPolicy, HSTSPolicy};
//!...
//!let uri_a = Uri::parse("https://www.google.com").unwrap();
//!let uri_b = Uri::parse("https://www.google.com").unwrap();
//!let helmet = Helmet::new()
//!                        //each policy has a default.
//!                        .hsts(HSTSPolicy::default())
//!                        //a header is turned off by setting its policy to None
//!                        .no_sniff(None)
//!                        .frameguard(FramePolicy::AllowFrom(uri_a))
//!                        .xss_protect(XSSPolicy::EnableReport(uri_b));
//!```
//!
//!
//!#### Still have questions?
//!
//!* <i>What policy should I choose? </i>Check out the documentation links above for individual headers on the Mozilla
//!  developer network, the [helmetjs](https://helmetjs.github.io/) doc's are also great
//!  resources.

extern crate rocket;

mod helmet;
mod policy;

pub use self::helmet::*;
pub use self::policy::*;
