use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

pub use rustls::server::{ClientHello, ServerConfig};

use crate::{Build, Ignite, Rocket};
use crate::fairing::{self, Info, Kind};

/// Proxy type to get PartialEq + Debug impls.
#[derive(Clone)]
pub struct DynResolver(Arc<dyn Resolver>);

pub struct Fairing<T: ?Sized>(PhantomData<T>);

/// A dynamic TLS configuration resolver.
///
/// # Example
///
/// This is an async trait. Implement it as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::sync::Arc;
/// use rocket::tls::{self, Resolver, TlsConfig, ClientHello, ServerConfig};
/// use rocket::{Rocket, Build};
///
/// struct MyResolver(Arc<ServerConfig>);
///
/// #[rocket::async_trait]
/// impl Resolver for MyResolver {
///     async fn init(rocket: &Rocket<Build>) -> tls::Result<Self> {
///         // This is equivalent to what the default resolver would do.
///         let config: TlsConfig = rocket.figment().extract_inner("tls")?;
///         let server_config = config.server_config().await?;
///         Ok(MyResolver(Arc::new(server_config)))
///     }
///
///     async fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<ServerConfig>> {
///         // return a `ServerConfig` based on `hello`; here we ignore it
///         Some(self.0.clone())
///     }
/// }
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().attach(MyResolver::fairing())
/// }
/// ```
#[crate::async_trait]
pub trait Resolver: Send + Sync + 'static {
    async fn init(rocket: &Rocket<Build>) -> crate::tls::Result<Self> where Self: Sized {
        let _rocket = rocket;
        let type_name = std::any::type_name::<Self>();
        Err(figment::Error::from(format!("{type_name}: Resolver::init() unimplemented")).into())
    }

    async fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<ServerConfig>>;

    fn fairing() -> Fairing<Self> where Self: Sized {
        Fairing(PhantomData)
    }
}

#[crate::async_trait]
impl<T: Resolver> fairing::Fairing for Fairing<T> {
    fn info(&self) -> Info {
        Info {
            name: "Resolver Fairing",
            kind: Kind::Ignite | Kind::Singleton
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let result = T::init(&rocket).await;
        match result {
            Ok(resolver) => Ok(rocket.manage(Arc::new(resolver) as Arc<dyn Resolver>)),
            Err(e) => {
                let type_name = std::any::type_name::<T>();
                error!(type_name, reason = %e, "TLS resolver failed to initialize");
                Err(rocket)
            }
        }
    }
}

impl DynResolver {
    pub fn extract(rocket: &Rocket<Ignite>) -> Option<Self> {
        rocket.state::<Arc<dyn Resolver>>().map(|r| Self(r.clone()))
    }
}

impl fmt::Debug for DynResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Resolver").finish()
    }
}

impl PartialEq for DynResolver {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Deref for DynResolver {
    type Target = dyn Resolver;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
