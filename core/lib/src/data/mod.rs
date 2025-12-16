//! Types and traits for handling incoming body data.

#[macro_use]
mod capped;
mod data;
mod data_stream;
mod from_data;
mod io_stream;
mod limits;
mod peekable;
mod transform;

pub use self::capped::{Capped, N};
pub use self::data::Data;
pub use self::data_stream::DataStream;
pub use self::from_data::{FromData, Outcome};
pub use self::io_stream::{IoHandler, IoStream};
pub use self::limits::Limits;
pub use self::transform::{Transform, TransformBuf};
pub use ubyte::{ByteUnit, ToByteUnit};

pub(crate) use self::data_stream::RawStream;
