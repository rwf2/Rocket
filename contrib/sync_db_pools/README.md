# `sync_db_pools` [![ci.svg]][ci] [![crates.io]][crate] [![docs.svg]][crate docs]

[crates.io]: https://img.shields.io/crates/v/rocket_sync_db_pools.svg
[crate]: https://crates.io/crates/rocket_sync_db_pools
[docs.svg]: https://img.shields.io/badge/web-master-red.svg?style=flat&label=docs&colorB=d33847
[crate docs]: https://api.rocket.rs/v0.5-rc/rocket_sync_db_pools
[ci.svg]: https://github.com/SergioBenitez/Rocket/workflows/CI/badge.svg
[ci]: https://github.com/SergioBenitez/Rocket/actions

This crate provides traits, utilities, and a procedural macro for configuring
and accessing database connection pools in Rocket. This implementation is backed
by [`r2d2`] and exposes connections through request guards.

[`r2d2`]: https://docs.rs/r2d2

## Usage

First, enable the feature corresponding to your database type:

```toml
[dependencies.rocket_sync_db_pools]
version = "0.1.0-rc.1"
features = ["diesel_sqlite_pool"]
```

A full list of supported databases and their associated feature names is
available in the [crate docs]. In whichever configuration source you choose,
configure a `databases` dictionary with a key for each database, here
`sqlite_logs` in a TOML source:

```toml
[default.databases]
sqlite_logs = { url = "/path/to/database.sqlite" }
```

In your application's source code:

```rust
#[macro_use] extern crate rocket;

use rocket_sync_db_pools::{database, diesel};

#[database("sqlite_logs")]
struct LogsDbConn(diesel::SqliteConnection);

#[get("/logs/<id>")]
async fn get_logs(conn: LogsDbConn, id: usize) -> Result<Logs> {
    conn.run(|c| Logs::by_id(c, id)).await
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(LogsDbConn::fairing())
}
```

See the [crate docs] for full details.
