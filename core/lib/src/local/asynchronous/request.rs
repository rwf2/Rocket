use std::borrow::Cow;

use crate::{Request, Data};
use crate::http::{Status, Method, uri::Origin, ext::IntoOwned};

use super::{Client, LocalResponse};

struct_request! { [
/// ## Example
///
/// The following snippet uses the available builder methods to construct a
/// `POST` request to `/` with a JSON body:
///
/// ```rust
/// use rocket::local::asynchronous::{Client, LocalRequest};
/// use rocket::http::{ContentType, Cookie};
///
/// # rocket::async_test(async {
/// let client = Client::new(rocket::ignite()).await.expect("valid rocket");
/// let req = client.post("/")
///     .header(ContentType::JSON)
///     .remote("127.0.0.1:8000".parse().unwrap())
///     .cookie(Cookie::new("name", "value"))
///     .body(r#"{ "value": 42 }"#);
/// # });
/// ```
]
pub struct LocalRequest<'c> {
    client: &'c Client,
    request: Request<'c>,
    data: Vec<u8>,
    uri: Cow<'c, str>,
}
}

impl<'c> LocalRequest<'c> {
    pub(crate) fn new(
        client: &'c Client,
        method: Method,
        uri: Cow<'c, str>
    ) -> LocalRequest<'c> {
        // We set a dummy string for now and check the user's URI on dispatch.
        let request = Request::new(client.rocket(), method, Origin::dummy());

        // Set up any cookies we know about.
        if let Some(ref jar) = client.cookies {
            let cookies = jar.read().expect("LocalRequest::new() read lock");
            for cookie in cookies.iter() {
                request.cookies().add_original(cookie.clone().into_owned());
            }
        }

        LocalRequest { client, request, uri, data: vec![] }
    }

    pub(crate) fn _request(&self) -> &Request<'c> {
        &self.request
    }

    pub(crate) fn _request_mut(&mut self) -> &mut Request<'c> {
        &mut self.request
    }

    pub(crate) fn _body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    // This method should _never_ be publicly exposed!
    #[inline(always)]
    fn long_lived_request<'a>(&mut self) -> &'a mut Request<'c> {
        // FIXME: Whatever. I'll kill this.
        unsafe { &mut *(&mut self.request as *mut _) }
    }

    // Performs the actual dispatch.
    // TODO.async: @jebrosen suspects there might be actual UB in here after all,
    //             and now we just went and mixed threads into it
    async fn _dispatch(mut self) -> LocalResponse<'c> {
        // First, validate the URI, returning an error response (generated from
        // an error catcher) immediately if it's invalid.
        if let Ok(uri) = Origin::parse(&self.uri) {
            self.request.set_uri(uri.into_owned());
        } else {
            error!("Malformed request URI: {}", self.uri);
            let res = self.client.rocket()
                .handle_error(Status::BadRequest, self.long_lived_request());

            return LocalResponse { _request: self.request, inner: res.await };
        }

        // Actually dispatch the request.
        let response = self.client.rocket()
            .dispatch(self.long_lived_request(), Data::local(self.data))
            .await;

        // If the client is tracking cookies, updates the internal cookie jar
        // with the changes reflected by `response`.
        if let Some(ref jar) = self.client.cookies {
            let mut jar = jar.write().expect("LocalRequest::_dispatch() write lock");
            let current_time = time::now();
            for cookie in response.cookies() {
                if let Some(expires) = cookie.expires() {
                    if expires <= current_time {
                        jar.force_remove(cookie);
                        continue;
                    }
                }

                jar.add(cookie.into_owned());
            }
        }

        LocalResponse { _request: self.request, inner: response }
    }
}

impl<'c> Clone for LocalRequest<'c> {
    fn clone(&self) -> Self {
        LocalRequest {
            client: self.client,
            request: self.request.clone(),
            data: self.data.clone(),
            uri: self.uri.clone(),
        }
    }
}

impl_request!("use rocket::local::asynchronous::Client;" @async await LocalRequest);
