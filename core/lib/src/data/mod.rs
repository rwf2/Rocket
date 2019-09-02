//! Types and traits for handling incoming body data.

mod data;
mod data_stream;
mod from_data;

pub use data::Data;
pub use data_stream::DataStream;
pub use from_data::{FromData, FromDataFuture, FromDataSimple, Outcome, Transform, Transformed, TransformFuture};
