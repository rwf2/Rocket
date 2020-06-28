use crate::{Request, Response};

struct_response! {
pub struct LocalResponse<'c> {
    pub(in super) _request: Request<'c>,
    pub(in super) inner: Response<'c>,
}
}

impl<'c> LocalResponse<'c> {
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
