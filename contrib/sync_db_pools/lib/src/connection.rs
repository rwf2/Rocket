use std::marker::PhantomData;
use std::sync::Arc;

use rocket::{Phase, Rocket, Ignite, Sentinel};
use rocket::fairing::{AdHoc, Fairing};
use rocket::request::{Request, Outcome, FromRequest};
use rocket::outcome::IntoOutcome;
use rocket::http::Status;

use rocket::tokio::sync::{OwnedSemaphorePermit, Semaphore, Mutex};
use rocket::tokio::time::timeout;

use crate::{Config, Poolable, Error};

/// Unstable internal details of generated code for the #[database] attribute.
///
/// This type is implemented here instead of in generated code to ensure all
/// types are properly checked.
#[doc(hidden)]
pub struct ConnectionPool<K, C: Poolable> {
    config: Config,
    // This is an 'Option' so that we can drop the pool in a 'spawn_blocking'.
    pool: Option<r2d2::Pool<C::Manager>>,
    semaphore: Arc<Semaphore>,
    _marker: PhantomData<fn() -> K>,
}

impl<K, C: Poolable> Clone for ConnectionPool<K, C> {
    fn clone(&self) -> Self {
        ConnectionPool {
            config: self.config.clone(),
            pool: self.pool.clone(),
            semaphore: self.semaphore.clone(),
            _marker: PhantomData
        }
    }
}

/// Unstable internal details of generated code for the #[database] attribute.
///
/// This type is implemented here instead of in generated code to ensure all
/// types are properly checked.
#[doc(hidden)]
pub struct Connection<K, C: Poolable> {
    connection: Arc<Mutex<Option<r2d2::PooledConnection<C::Manager>>>>,
    permit: Option<OwnedSemaphorePermit>,
    _marker: PhantomData<fn() -> K>,
}

// A wrapper around spawn_blocking that propagates panics to the calling code.
async fn run_blocking<F, R>(job: F) -> R
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static,
{
    match tokio::task::spawn_blocking(job).await {
        Ok(ret) => ret,
        Err(e) => match e.try_into_panic() {
            Ok(panic) => std::panic::resume_unwind(panic),
            Err(_) => unreachable!("spawn_blocking tasks are never cancelled"),
        }
    }
}

macro_rules! dberr {
    ($msg:literal, $db_name:expr, $efmt:literal, $error:expr, $rocket:expr) => ({
        rocket::error!(concat!("database ", $msg, " error for pool named `{}`"), $db_name);
        error_!($efmt, $error);
        return Err($rocket);
    });
}

impl<K: 'static, C: Poolable> ConnectionPool<K, C> {
    pub fn fairing(fairing_name: &'static str, db: &'static str) -> impl Fairing {
        AdHoc::try_on_ignite(fairing_name, move |rocket| async move {
            run_blocking(move || {
                let config = match Config::from(db, &rocket) {
                    Ok(config) => config,
                    Err(e) => dberr!("config", db, "{}", e, rocket),
                };

                let pool_size = config.pool_size;
                match C::pool(db, &rocket) {
                    Ok(pool) => Ok(rocket.manage(ConnectionPool::<K, C> {
                        config,
                        pool: Some(pool),
                        semaphore: Arc::new(Semaphore::new(pool_size as usize)),
                        _marker: PhantomData,
                    })),
                    Err(Error::Config(e)) => dberr!("config", db, "{}", e, rocket),
                    Err(Error::Pool(e)) => dberr!("pool init", db, "{}", e, rocket),
                    Err(Error::Custom(e)) => dberr!("pool manager", db, "{:?}", e, rocket),
                }
            }).await
        })
    }

    pub async fn get(&self) -> Option<Connection<K, C>> {
        let duration = std::time::Duration::from_secs(self.config.timeout as u64);
        let permit = match timeout(duration, self.semaphore.clone().acquire_owned()).await {
            Ok(p) => p.expect("internal invariant broken: semaphore should not be closed"),
            Err(_) => {
                error_!("database connection retrieval timed out");
                return None;
            }
        };

        let pool = self.pool.as_ref().cloned()
            .expect("internal invariant broken: self.pool is Some");

        match run_blocking(move || pool.get_timeout(duration)).await {
            Ok(c) => Some(Connection {
                connection: Arc::new(Mutex::new(Some(c))),
                permit: Some(permit),
                _marker: PhantomData,
            }),
            Err(e) => {
                error_!("failed to get a database connection: {}", e);
                None
            }
        }
    }

    #[inline]
    pub async fn get_one<P: Phase>(rocket: &Rocket<P>) -> Option<Connection<K, C>> {
        match Self::pool(rocket) {
            Some(pool) => match pool.get().await {
                Some(conn) => Some(conn),
                None => {
                    error_!("no connections available for `{}`", std::any::type_name::<K>());
                    None
                }
            },
            None => {
                error_!("missing database fairing for `{}`", std::any::type_name::<K>());
                None
            }
        }
    }

    #[inline]
    pub fn pool<P: Phase>(rocket: &Rocket<P>) -> Option<&Self> {
        rocket.state::<Self>()
    }
}

impl<K: 'static, C: Poolable> Connection<K, C> {
    pub async fn run<F, R>(&self, f: F) -> R
        where F: FnOnce(&mut C) -> R + Send + 'static,
              R: Send + 'static,
    {
        // It is important that this inner Arc<Mutex<>> (or the OwnedMutexGuard
        // derived from it) never be a variable on the stack at an await point,
        // where Drop might be called at any time. This causes (synchronous)
        // Drop to be called from asynchronous code, which some database
        // wrappers do not or can not handle.
        let connection = self.connection.clone();

        // Since connection can't be on the stack in an async fn during an
        // await, we have to spawn a new blocking-safe thread...
        run_blocking(move || {
            // And then re-enter the runtime to wait on the async mutex, but in
            // a blocking fashion.
            let mut connection = tokio::runtime::Handle::current().block_on(async {
                connection.lock_owned().await
            });

            let conn = connection.as_mut()
                .expect("internal invariant broken: self.connection is Some");
            f(conn)
        }).await
    }
}

impl<K, C: Poolable> Drop for Connection<K, C> {
    fn drop(&mut self) {
        let connection = self.connection.clone();
        let permit = self.permit.take();

        // See same motivation above for this arrangement of spawn_blocking/block_on
        tokio::task::spawn_blocking(move || {
            let mut connection = tokio::runtime::Handle::current().block_on(async {
                connection.lock_owned().await
            });

            if let Some(conn) = connection.take() {
                drop(conn);
            }

            // Explicitly dropping the permit here so that it's only
            // released after the connection is.
            drop(permit);
        });
    }
}

impl<K, C: Poolable> Drop for ConnectionPool<K, C> {
    fn drop(&mut self) {
        let pool = self.pool.take();
        tokio::task::spawn_blocking(move || drop(pool));
    }
}

#[rocket::async_trait]
impl<'r, K: 'static, C: Poolable> FromRequest<'r> for Connection<K, C> {
    type Error = ();

    #[inline]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        match request.rocket().state::<ConnectionPool<K, C>>() {
            Some(c) => c.get().await.into_outcome((Status::ServiceUnavailable, ())),
            None => {
                error_!("Missing database fairing for `{}`", std::any::type_name::<K>());
                Outcome::Failure((Status::InternalServerError, ()))
            }
        }
    }
}

impl<K: 'static, C: Poolable> Sentinel for Connection<K, C> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        use rocket::yansi::Paint;

        if rocket.state::<ConnectionPool<K, C>>().is_none() {
            let conn = Paint::default(std::any::type_name::<K>()).bold();
            let fairing = Paint::default(format!("{}::fairing()", conn)).wrap().bold();
            error!("requesting `{}` DB connection without attaching `{}`.", conn, fairing);
            info_!("Attach `{}` to use database connection pooling.", fairing);
            return true;
        }

        false
    }
}
