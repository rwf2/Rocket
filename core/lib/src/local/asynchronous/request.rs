use std::borrow::Cow;

use crate::{Request, Data};
use crate::http::{Status, Method, uri::Origin, ext::IntoOwned};

use super::{Client, LocalResponse};

/// An `async` local request as returned by [`Client`](super::Client).
///
/// For details, see [the top-level documentation](../index.html#localrequest).
///
/// ## Example
///
/// The following snippet uses the available builder methods to construct and
/// dispatch a `POST` request to `/` with a JSON body:
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
///
/// let response = req.dispatch().await;
/// # });
/// ```
pub struct LocalRequest<'c> {
    client: &'c Client,
    request: Request<'c>,
    data: Vec<u8>,
    uri: Cow<'c, str>,
}

impl<'c> LocalRequest<'c> {
    pub(crate) fn new(
        client: &'c Client,
        method: Method,
        uri: Cow<'c, str>
    ) -> LocalRequest<'c> {
        // We try to validate the URI now so that the inner `Request` contains a
        // valid URI. If it doesn't, we set a dummy one.
        let origin = Origin::parse(&uri).unwrap_or_else(|_| Origin::dummy());
        let request = Request::new(client.rocket(), method, origin.into_owned());

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

    // Performs the actual dispatch.
    async fn _dispatch(mut self) -> LocalResponse<'c> {
        // First, revalidate the URI, returning an error response (generated
        // from an error catcher) immediately if it's invalid. If it's valid,
        // then `request` already contains the correct URI.
        let rocket = self.client.rocket();
        if let Err(_) = Origin::parse(&self.uri) {
            error!("Malformed request URI: {}", self.uri);
            return LocalResponse::new(self.request, move |req| {
                rocket.handle_error(Status::BadRequest, req)
            }).await
        }

        // Actually dispatch the request.
        let mut data = Data::local(self.data);
        let token = rocket.preprocess_request(&mut self.request, &mut data).await;
        let response = LocalResponse::new(self.request, move |request| {
            rocket.dispatch(token, request, data)
        }).await;

        // If the client is tracking cookies, updates the internal cookie jar
        // with the changes reflected by `response`.
        if let Some(ref jar) = self.client.cookies {
            let mut jar = jar.write().expect("LocalRequest::_dispatch() write lock");
            let current_time = time::OffsetDateTime::now_utc();
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

        response
    }

    pub_request_impl!("# use rocket::local::asynchronous::Client;
        use rocket::local::asynchronous::LocalRequest;" async await);
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

impl std::fmt::Debug for LocalRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._request().fmt(f)
    }
}
