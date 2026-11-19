// pub mod session;
mod client;
mod server;

pub use server::*;
pub use client::*;

use std::io;
use thiserror::Error;
use procspawn::SpawnError;
use rocket::http::uri;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("join/kill failed: {0}")]
    JoinError(#[from] SpawnError),
    #[error("kill failed: {0}")]
    TermFailure(#[from] nix::errno::Errno),
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid URI: {0}")]
    Uri(#[from] uri::Error<'static>),
    #[error("invalid uri: {0}")]
    InvalidUri(uri::Uri<'static>),
    #[error("expected certificates are not present")]
    MissingCertificate,
    #[error("invalid PEM: {0}")]
    Pem(#[from] rustls_pki_types::pem::Error),
    #[error("bad request: {0}")]
    Request(#[from] reqwest::Error),
    #[error("IPC failure: {0}")]
    Ipc(#[from] ipc_channel::IpcError),
    #[error("liftoff failed")]
    Liftoff(String, String),
}
