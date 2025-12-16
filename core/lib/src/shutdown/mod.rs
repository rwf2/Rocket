//! Shutdown configuration and notification handle.

mod config;
mod handle;
mod sig;
mod tripwire;

pub(crate) use handle::Stages;
pub(crate) use tripwire::TripWire;

pub use config::ShutdownConfig;
pub use handle::Shutdown;
pub use sig::Sig;
