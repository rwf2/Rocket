use std::future::Future;

use crate::{Request, Response};

struct_response! {
pub struct LocalResponse<'c> {
    pub(in super) _request: Box<Request<'c>>,
    pub(in super) inner: Response<'c>,
}
}

impl<'c> LocalResponse<'c> {
    pub(crate) fn new<F, O>(req: Request<'c>, f: F) -> impl Future<Output = LocalResponse<'c>> + 'c
        where F: FnOnce(&'c Request<'c>) -> O + 'c + Send,
              O: Future<Output = Response<'c>> + Send
    {
        let boxed_req = Box::new(req);
        let request: &'c Request<'c> = unsafe { &*(&*boxed_req as *const _) };
        async move {
            LocalResponse {
                _request: boxed_req,
                inner: f(request).await
            }
        }
    }

    fn _response(&self) -> &Response<'c> {
        &self.inner
    }

    pub(crate) async fn _into_string(mut self) -> Option<String> {
        self.inner.body_string().await
    }

    pub(crate) async fn _into_bytes(mut self) -> Option<Vec<u8>> {
        self.inner.body_bytes().await
    }
}

impl_response!("use rocket::local::asynchronous::Client;" @async await LocalResponse);
