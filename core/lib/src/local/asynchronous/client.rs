use std::sync::RwLock;
use std::borrow::Cow;

use crate::local::asynchronous::LocalRequest;
use crate::rocket::{Rocket, Cargo};
use crate::http::{Method, private::CookieJar};
use crate::error::LaunchError;

pub struct Client {
    cargo: Cargo,
    pub(crate) cookies: Option<RwLock<CookieJar>>,
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
