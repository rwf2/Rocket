use rocket::fairing::{Info, Kind};
use rocket::futures::future::BoxFuture;
use rocket::{Build, Ignite, Rocket, Sentinel};

use crate::Pool;

/// Trait implemented to define a database connection pool.
pub trait Database: Sized + Send + Sync + 'static {
    /// The name of this connection pool in the configuration.
    const NAME: &'static str;

    /// The underlying connection type returned by this pool.
    /// Must implement [`Pool`].
    type Pool: Pool;

    /// Returns a fairing that attaches this connection pool to the server.
    fn fairing() -> Fairing<Self>;

    /// Direct shared access to the underlying database pool
    fn pool(&self) -> &Self::Pool;

    /// get().await returns a connection from the pool (or an error)
    fn get(&self) -> BoxFuture<'_, Result<Connection<Self>, <Self::Pool as Pool>::GetError>> {
        Box::pin(async move { self.pool().get().await.map(Connection)} )
    }
}

/// A connection. The underlying connection type is determined by `D`, which
/// must implement [`Database`].
pub struct Connection<D: Database>(<D::Pool as Pool>::Connection);

impl<D: Database> std::ops::Deref for Connection<D> {
    type Target = <D::Pool as Pool>::Connection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Database> std::ops::DerefMut for Connection<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl<'r, D: Database> rocket::request::FromRequest<'r> for Connection<D> {
    type Error = <D::Pool as Pool>::GetError;

    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        use rocket::http::Status;
        use rocket::request::Outcome;

        let db: &D = match req.rocket().state() {
            Some(p) => p,
            _ => panic!(
                "tried to use database connection pool {}, but its fairing was not attached",
                D::NAME,
            ),
        };

        match db.pool().get().await {
            Ok(conn) => Outcome::Success(Connection(conn)),
            Err(e) => Outcome::Failure((Status::ServiceUnavailable, e)),
        }
    }
}

impl<D: Database> Sentinel for Connection<D> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        use rocket::yansi::Paint;

        if rocket.state::<D>().is_none() {
            let dbtype = Paint::default(std::any::type_name::<D>()).bold();
            let fairing = Paint::default(format!("{}::fairing()", dbtype)).wrap().bold();
            error!("requesting `{}` DB connection without attaching `{}`.", dbtype, fairing);
            info_!("Attach `{}` to use database connection pooling.", fairing);
            return true;
        }

        false
    }
}

/// The database fairing for pool types created with the `pool!` macro.
pub struct Fairing<D: Database>(fn(D::Pool) -> D);

impl<D: Database> Fairing<D> {
    /// Create a new database fairing with the given constructor.  This
    /// constructor will be called to create an instance of `D` after the pool
    /// is initialized and before it is placed into managed state.
    pub fn new(ctor: fn(D::Pool) -> D) -> Self {
        Self(ctor)
    }
}

#[rocket::async_trait]
impl<D: Database> rocket::fairing::Fairing for Fairing<D> {
    fn info(&self) -> Info {
        Info {
            name: "rocket_db_pools connection pool",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
        let db_config = match rocket
            .figment()
            .find_value(&format!("databases.{}", D::NAME))
        {
            Ok(v) => v,
            Err(e) => {
                error!("error getting database configuration: {}", e);
                return Err(rocket);
            }
        };

        let config = match db_config.deserialize() {
            Ok(c) => c,
            Err(e) => {
                error!("error deserializing configuration: {}", e);
                return Err(rocket);
            }
        };

        let pool = match <D::Pool>::initialize(config).await {
            Ok(p) => p,
            Err(e) => {
                error!("error initializing database connection pool: {}", e);
                return Err(rocket);
            }
        };

        Ok(rocket.manage((self.0)(pool)))
    }
}
