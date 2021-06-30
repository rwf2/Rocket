use std::fmt;
use std::convert::TryInto;

use parking_lot::RwLock;

use crate::{Rocket, Phase, Orbit, Error};
use crate::local::asynchronous::{LocalRequest, LocalResponse};
use crate::http::{Method, uri::Origin, private::cookie};

/// An `async` client to construct and dispatch local requests.
///
/// For details, see [the top-level documentation](../index.html#client).
/// For the `blocking` version, see
/// [`blocking::Client`](crate::local::blocking::Client).
///
/// ## Multithreaded Tracking Synchronization Pitfalls
///
/// Unlike its [`blocking`](crate::local::blocking) variant, this `async`
/// `Client` implements `Sync`. However, using it in a multithreaded environment
/// while tracking cookies can result in surprising, non-deterministic behavior.
/// This is because while cookie modifications are serialized, the ordering
/// depends on the ordering of request dispatch.
///
/// If possible, refrain from sharing a single instance of a tracking `Client`
/// across multiple threads. Instead, prefer to create a unique instance of
/// `Client` per thread. If this is not possible, ensure that you are not
/// depending on the ordering of cookie modifications or have arranged for
/// request dispatch to occur in a deterministic manner.
///
/// Alternatively, use an untracked client, which does not suffer from these
/// pitfalls.
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local `POST /` request with a body of `Hello, world!`.
///
/// ```rust,no_run
/// use rocket::local::asynchronous::Client;
///
/// # rocket::async_test(async {
/// let rocket = rocket::build();
/// let client = Client::tracked(rocket).await.expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch()
///     .await;
/// # });
/// ```
pub struct Client {
    rocket: Rocket<Orbit>,
    cookies: RwLock<cookie::CookieJar>,
    pub(in super) tracked: bool,
}

impl Client {
    pub(crate) async fn _new<P: Phase>(
        rocket: Rocket<P>,
        tracked: bool
    ) -> Result<Client, Error> {
        let rocket = rocket.local_launch().await?;
        let cookies = RwLock::new(cookie::CookieJar::new());
        Ok(Client { rocket, cookies, tracked })
    }

    // WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn _test<T, F>(f: F) -> T
        where F: FnOnce(&Self, LocalRequest<'_>, LocalResponse<'_>) -> T + Send
    {
        crate::async_test(async {
            let client = Client::debug(crate::build()).await.unwrap();
            let request = client.get("/");
            let response = request.clone().dispatch().await;
            f(&client, request, response)
        })
    }

    #[inline(always)]
    pub(crate) fn _rocket(&self) -> &Rocket<Orbit> {
        &self.rocket
    }

    #[inline(always)]
    pub(crate) fn _with_raw_cookies<F, T>(&self, f: F) -> T
        where F: FnOnce(&cookie::CookieJar) -> T
    {
        f(&*self.cookies.read())
    }

    #[inline(always)]
    pub(crate) fn _with_raw_cookies_mut<F, T>(&self, f: F) -> T
        where F: FnOnce(&mut cookie::CookieJar) -> T
    {
        f(&mut *self.cookies.write())
    }

    #[inline(always)]
    fn _req<'c, 'u: 'c, U>(&'c self, method: Method, uri: U) -> LocalRequest<'c>
        where U: TryInto<Origin<'u>> + fmt::Display
    {
        LocalRequest::new(self, method, uri)
    }

    // Generates the public API methods, which call the private methods above.
    pub_client_impl!("use rocket::local::asynchronous::Client;" @async await);
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._rocket().fmt(f)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_local_client_impl_send_sync() {
        fn assert_sync_send<T: Sync + Send>() {}
        assert_sync_send::<super::Client>();
    }
}
