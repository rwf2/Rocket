use std::sync::RwLock;
use std::borrow::Cow;

use crate::local::asynchronous::{LocalRequest, LocalResponse};
use crate::rocket::{Rocket, Cargo};
use crate::http::{Method, private::CookieJar};
use crate::error::LaunchError;

/// An `async` client to construct and dispatch local requests.
///
/// For details, see [the top-level documentation](../index.html#client).
/// For the `blocking` version, see
/// [`blocking::Client`](crate::local::blocking::Client).
///
/// ## Multithreaded Syncronization Pitfalls
///
/// Unlike its [`blocking`](crate::local::blocking) variant, this `async` `Client`
/// implements `Sync`. However, using it in a multithreaded environment while
/// tracking cookies can result in surprising, non-deterministic behavior. This
/// is because while cookie modifications are serialized, the exact ordering
/// depends on when requests are dispatched. Specifically, when cookie tracking
/// is enabled, all request dispatches are serialized, which in-turn serializes
/// modifications to the internally tracked cookies.
///
/// If possible, refrain from sharing a single instance of `Client` across
/// multiple threads. Instead, prefer to create a unique instance of `Client`
/// per thread. If it's not possible, ensure that either you are not depending
/// on cookies, the ordering of their modifications, or both, or have arranged
/// for dispatches to occur in a deterministic ordering.
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local `POST /` request with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::asynchronous::Client;
///
/// # rocket::async_test(async {
/// let rocket = rocket::ignite();
/// let client = Client::new(rocket).await.expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch()
///     .await;
/// # });
/// ```
pub struct Client {
    cargo: Cargo,
    pub(in super) cookies: Option<RwLock<CookieJar>>,
}

impl Client {
    pub(crate) async fn _new(
        mut rocket: Rocket,
        tracked: bool
    ) -> Result<Client, LaunchError> {
        rocket.prelaunch_check().await?;

        let cookies = match tracked {
            true => Some(RwLock::new(CookieJar::new())),
            false => None
        };

        Ok(Client { cargo: rocket.into_cargo().await, cookies })
    }

    // WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn _test<T, F>(f: F) -> T
        where F: FnOnce(&Self, LocalRequest<'_>, LocalResponse<'_>) -> T + Send
    {
        crate::async_test(async {
            let rocket = crate::ignite();
            let client = Client::new(rocket).await.expect("valid rocket");
            let request = client.get("/");
            let response = request.clone().dispatch().await;
            f(&client, request, response)
        })
    }

    #[inline(always)]
    pub(crate) fn _cargo(&self) -> &Cargo {
        &self.cargo
    }

    #[inline(always)]
    fn _req<'c, 'u: 'c, U>(&'c self, method: Method, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        LocalRequest::new(self, method, uri.into())
    }

    // Generates the public API methods, which call the private methods above.
    pub_client_impl!("use rocket::local::asynchronous::Client;" @async await);
}

#[cfg(test)]
mod test {
    #[test]
    fn test_local_client_impl_send_sync() {
        fn assert_sync_send<T: Sync + Send>() {}
        assert_sync_send::<super::Client>();
    }
}
