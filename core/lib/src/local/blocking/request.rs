use std::borrow::Cow;

use crate::{Request, http::Method, local::asynchronous};

use super::{Client, LocalResponse};
#[derive(Clone)]
pub struct LocalRequest<'c> {
    inner: asynchronous::LocalRequest<'c>,
    client: &'c Client,
}

impl<'c> LocalRequest<'c> {
    #[inline]
    pub(crate) fn new(
        client: &'c Client,
        method: Method,
        uri: Cow<'c, str>
    ) -> LocalRequest<'c> {
        let inner = asynchronous::LocalRequest::new(&client.inner, method, uri);
        Self { inner, client }
    }

    #[inline]
    fn _request(&self) -> &Request<'c> {
        self.inner._request()
    }

    #[inline]
    fn _request_mut(&mut self) -> &mut Request<'c> {
        self.inner._request_mut()
    }

    fn _body_mut(&mut self) -> &mut Vec<u8> {
        self.inner._body_mut()
    }

    fn _dispatch(self) -> LocalResponse<'c> {
        let inner = self.client.block_on(self.inner.dispatch());
        LocalResponse { inner, client: self.client }
    }
}

impl_request!("use rocket::local::blocking::Client;" LocalRequest);
