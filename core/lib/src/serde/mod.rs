//! Serialization and deserialization support.
//!
//! * JSON support is provided by the [`Json`](json::Json) type.
//! * MessagePack support is provided by the [`MsgPack`](msgpack::MsgPack) type.
//! * UUID support is provided by the [`UUID`](uuid) type.
//!
//! Types implement one or all of [`FromParam`](crate::request::FromParam),
//! [`FromForm`](crate::form::FromForm), [`FromData`](crate::data::FromData),
//! and [`Responder`](crate::response::Responder).
//!
//! ## Deriving `Serialize`, `Deserialize`
//!
//! For convenience, Rocket re-exports `serde`'s `Serialize` and `Deserialize`
//! traits and derive macros from this module. However, due to Rust's limited
//! support for derive macro re-exports, using the re-exported derive macros
//! requires annotating structures with `#[serde(crate = "rocket::serde")]`:
//!
//! ```rust
//! use rocket::serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[serde(crate = "rocket::serde")]
//! struct MyStruct {
//!     foo: String,
//! }
//! ```
//!
//! If you'd like to avoid this extra annotation, you must depend on `serde`
//! directly via your crate's `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! ```

#[doc(inline)]
pub use serde::ser::{Serialize, Serializer};

#[doc(inline)]
pub use serde::de::{Deserialize, DeserializeOwned, Deserializer};

#[doc(hidden)]
pub use serde::*;

#[cfg(feature = "json")]
#[cfg_attr(nightly, doc(cfg(feature = "json")))]
pub mod json;

#[cfg(feature = "msgpack")]
#[cfg_attr(nightly, doc(cfg(feature = "msgpack")))]
pub mod msgpack;

#[cfg(feature = "uuid")]
#[cfg_attr(nightly, doc(cfg(feature = "uuid")))]
pub mod uuid;
