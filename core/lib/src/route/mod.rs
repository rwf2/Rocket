//! Types and traits for routes and their request handlers and return types.

mod handler;
mod route;
mod segment;
mod uri;

pub use handler::*;
pub use route::*;
pub use uri::*;

pub(crate) use segment::Segment;
