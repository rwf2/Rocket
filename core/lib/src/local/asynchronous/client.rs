use std::sync::RwLock;
use std::borrow::Cow;

use crate::local::asynchronous::LocalRequest;
use crate::rocket::{Rocket, Cargo};
use crate::http::{Method, private::CookieJar};
use crate::error::LaunchError;

struct_client! { [
///
/// ### Synchronization
///
/// While `Client` implements `Sync`, using it in a multithreaded environment
/// while tracking cookies can result in surprising, non-deterministic behavior.
/// This is because while cookie modifications are serialized, the exact
/// ordering depends on when requests are dispatched. Specifically, when cookie
/// tracking is enabled, all request dispatches are serialized, which in-turn
/// serializes modifications to the internally tracked cookies.
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
/// dispatches a local request to `POST /` with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::asynchronous::Client;
///
/// # rocket::async_test(async {
/// let rocket = rocket::ignite();
/// let client = Client::new(rocket).await.expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch().await;
/// # });
/// ```
]
pub struct Client {
    cargo: Cargo,
    pub(crate) cookies: Option<RwLock<CookieJar>>,
}
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

    #[doc(hidden)]
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    pub fn _test<T, F: FnOnce(Self) -> T + Send>(f: F) -> T {
        crate::async_test(async {
            let rocket = crate::ignite();
            let client = Client::new(rocket).await.expect("valid rocket");
            f(client)
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
}

impl_client!("use rocket::local::asynchronous::Client;" @async await Client);

#[cfg(test)]
mod test {
    use super::Client;

    fn assert_sync_send<T: Sync + Send>() {}

    #[test]
    fn test_local_client_impl_send_sync() {
        assert_sync_send::<Client>();
    }
}
