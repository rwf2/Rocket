use crate::{Response, local::asynchronous};

use super::Client;

struct_response! {
pub struct LocalResponse<'c> {
    pub(in super) inner: asynchronous::LocalResponse<'c>,
    pub(in super) client: &'c Client,
}
}

impl<'c> LocalResponse<'c> {
    fn _response(&self) -> &Response<'c> {
        &*self.inner
    }

    fn _into_string(self) -> Option<String> {
        self.client.block_on(self.inner._into_string())
    }

    fn _into_bytes(self) -> Option<Vec<u8>> {
        self.client.block_on(self.inner._into_bytes())
    }
}

impl_response!("use rocket::local::blocking::Client;" LocalResponse);
