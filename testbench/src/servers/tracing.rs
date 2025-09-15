//! Check that guard failures result in trace with `Display` message for guard
//! types that implement `Display` and otherwise uses `Debug`.

use std::fmt;

use rocket::data::{self, FromData};
use rocket::http::uri::{Segments, fmt::Path};
use rocket::request::{self, FromParam, FromRequest, FromSegments};

use crate::prelude::*;

#[derive(Debug, TypedError)]
struct UseDisplay(&'static str);

#[derive(Debug, TypedError)]
struct UseDebug;

impl fmt::Display for UseDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "this is the display impl: {}", self.0)
    }
}

impl FromParam<'_> for UseDisplay {
    type Error = Self;
    fn from_param(_: &str) -> Result<Self, Self::Error> { Err(Self("param")) }
}

impl FromParam<'_> for UseDebug {
    type Error = Self;
    fn from_param(_: &str) -> Result<Self, Self::Error> { Err(Self) }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UseDisplay {
    type Forward = Self;
    type Error = Self;
    async fn from_request(_: &'r Request<'_>) ->
        request::Outcome<Self, Self::Error, Self::Forward>
    {
        request::Outcome::Error(Self("req"))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UseDebug {
    type Forward = Self;
    type Error = Self;
    async fn from_request(_: &'r Request<'_>) ->
        request::Outcome<Self, Self::Error, Self::Forward>
    {
        request::Outcome::Error(Self)
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UseDisplay {
    type Error = Self;
    async fn from_data(_: &'r Request<'_>, _: Data<'r>) -> data::Outcome<'r, Self> {
        data::Outcome::Error(Self("data"))
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UseDebug {
    type Error = Self;
    async fn from_data(_: &'r Request<'_>, _: Data<'r>) -> data::Outcome<'r, Self> {
        data::Outcome::Error(Self)
    }
}

impl<'r> FromSegments<'r> for UseDisplay {
    type Error = Self;
    fn from_segments(_: Segments<'r, Path>) -> Result<Self, Self::Error> { Err(Self("segment")) }
}

impl<'r> FromSegments<'r> for UseDebug {
    type Error = Self;
    fn from_segments(_: Segments<'r, Path>) -> Result<Self, Self::Error> { Err(Self) }
}

pub fn test_display_guard_err() -> Result<()> {
    #[get("/<_v>", rank = 1)] fn a(_v: UseDisplay) {}
    #[get("/<_v..>", rank = 2)] fn b(_v: UseDisplay) {}
    #[get("/<_..>", rank = 3)] fn d(_v: UseDisplay) {}
    #[post("/<_..>", data = "<_v>")] fn c(_v: UseDisplay) {}

    let mut server = spawn! {
        Rocket::default().mount("/", routes![a, b, c, d])
    }?;

    let client = Client::default();
    client.get(&server, "/foo")?.send()?;
    client.post(&server, "/foo")?.send()?;
    server.terminate()?;

    let stdout = server.read_stdout()?;
    assert!(stdout.contains("this is the display impl: param"));
    assert!(stdout.contains("this is the display impl: req"));
    assert!(stdout.contains("this is the display impl: segment"));
    assert!(stdout.contains("this is the display impl: data"));

    Ok(())
}

pub fn test_debug_guard_err() -> Result<()> {
    #[get("/<_v>", rank = 1)] fn a(_v: UseDebug) {}
    #[get("/<_v..>", rank = 2)] fn b(_v: UseDebug) {}
    #[get("/<_..>", rank = 3)] fn d(_v: UseDebug) {}
    #[post("/<_..>", data = "<_v>")] fn c(_v: UseDebug) {}

    let mut server = spawn! {
        Rocket::default().mount("/", routes![a, b, c, d])
    }?;

    let client = Client::default();
    client.get(&server, "/foo")?.send()?;
    client.post(&server, "/foo")?.send()?;
    server.terminate()?;

    let stdout = server.read_stdout()?;
    assert!(!stdout.contains("this is the display impl"));
    assert!(stdout.contains("UseDebug"));
    Ok(())
}

register!(test_display_guard_err);
register!(test_debug_guard_err);
