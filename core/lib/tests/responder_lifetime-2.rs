#![feature(proc_macro_hygiene)]
#![allow(dead_code)] // This test is only here so that we can ensure it compiles.

use rocket::handler::{Handler, HandlerFuture, Outcome};
use rocket::http::Method;
use rocket::response::Responder;
use rocket::{Data, Request, Route};

/// A "reusable responder" that can be mounted anywhere via 'into_route'.
#[derive(Clone)]
pub struct ReusableResponder<R>(R);

impl<R: Responder<'static> + Clone + Send + Sync + 'static> ReusableResponder<R> {
    pub fn into_route(self, path: impl AsRef<str>) -> Route {
        Route::new(Method::Get, path, self)
    }
}

impl<R: Responder<'static> + Clone + Send + Sync + 'static> Handler for ReusableResponder<R> {
    fn handle<'r>(&self, req: &'r Request, _: Data) -> HandlerFuture<'r> {
        let responder = self.0.clone();
        Box::pin(async move {
            match responder.respond_to(req).await {
                Ok(response) => Outcome::Success(response),
                Err(status) => Outcome::Failure(status)
            }
        })
    }
}
