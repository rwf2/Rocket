use request::Request;
use response::{Response, Responder};
use http::Status;

/// A failing response; simply forwards to the catcher for the given
/// `Status`.
#[derive(Debug)]
pub struct Failure(pub Status);

impl Responder<'static> for Failure {
    fn respond_to(self, _: &Request) -> Result<Response<'static>, Status> {
        Err(self.0)
    }
}

impl From<Status> for Failure {
    fn from(status: Status) -> Self {
        Failure(status)
    }
}
