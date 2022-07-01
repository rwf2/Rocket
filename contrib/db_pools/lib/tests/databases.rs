macro_rules! check_types_match {
    ($feature:expr, $name:ident, $Pool:ty, $Conn:ty $(,)?) => (
        #[cfg(feature = $feature)]
        mod $name {
            use rocket::*;
            use rocket_db_pools::{Connection, Database};

            #[derive(Database)]
            #[database("foo")]
            struct Db($Pool);

            #[get("/")]
            fn _db(conn: Connection<Db>) {
                let _: &$Conn = &*conn;
            }
        }
    )
}

check_types_match!(
    "deadpool_postgres",
    deadpool_postgres,
    deadpool_postgres::Pool,
    deadpool_postgres::ClientWrapper,
);

check_types_match!(
    "deadpool_redis",
    deadpool_redis,
    deadpool_redis::Pool,
    deadpool_redis::Connection,
);

check_types_match!(
    "sqlx_postgres",
    sqlx_postgres,
    sqlx::PgPool,
    sqlx::pool::PoolConnection<sqlx::Postgres>,
);

check_types_match!(
    "sqlx_mysql",
    sqlx_mysql,
    sqlx::MySqlPool,
    sqlx::pool::PoolConnection<sqlx::MySql>,
);

check_types_match!(
    "sqlx_sqlite",
    sqlx_sqlite,
    sqlx::SqlitePool,
    sqlx::pool::PoolConnection<sqlx::Sqlite>,
);

check_types_match!(
    "sqlx_mssql",
    sqlx_mssql,
    sqlx::MssqlPool,
    sqlx::pool::PoolConnection<sqlx::Mssql>,
);

check_types_match!(
    "mongodb",
    mongodb,
    mongodb::Client,
    mongodb::Client,
);
