# `db_pools` [![ci.svg]][ci] [![crates.io]][crate] [![docs.svg]][crate docs]

[crates.io]: https://img.shields.io/crates/v/rocket_db_pools.svg
[crate]: https://crates.io/crates/rocket_db_pools
[docs.svg]: https://img.shields.io/badge/web-master-red.svg?style=flat&label=docs&colorB=d33847
[crate docs]: https://api.rocket.rs/v0.5-rc/rocket_db_pools
[ci.svg]: https://github.com/SergioBenitez/Rocket/workflows/CI/badge.svg
[ci]: https://github.com/SergioBenitez/Rocket/actions

Asynchronous database driver integration for Rocket. See the [crate docs] for
full usage details.

## Usage

1. Add `rocket_db_pools` as a dependency with one or more [database driver
   features] enabled:

   ```toml
   [dependencies.rocket_db_pools]
   version = "0.1.0-rc.2"
   features = ["sqlx_sqlite"]
   ```

2. Choose a name for your database, here `sqlite_logs`. [Configure] _at least_ a
   URL for the database:

   ```toml
   [default.databases.sqlite_logs]
   url = "/path/to/database.sqlite"
   ```

3. [Derive `Database`] for a unit type (`Logs` here) which
   wraps the selected driver's [`Pool`] type and is decorated with
   `#[database("name")]`. Attach `Type::init()` to your application's `Rocket`
   to initialize the database pool:

   ```rust
   use rocket_db_pools::{Database, Connection};

   #[derive(Database)]
   #[database("sqlite_logs")]
   struct Logs(sqlx::SqlitePool);

   #[launch]
   fn rocket() -> _ {
       rocket::build().attach(Logs::init())
   }
   ```

4. Use [`Connection<Type>`] as a request guard to retrieve an
   active database connection:

   ```rust
   #[get("/<id>")]
   async fn read(mut db: Connection<Logs>, id: i64) -> Result<Log> {
       sqlx::query!("SELECT content FROM logs WHERE id = ?", id)
           .fetch_one(&mut *db)
           .map_ok(|r| Log(r.content))
           .await
   }
   ```

[database driver features]: https://api.rocket.rs/v0.5-rc/rocket_db_pools/index.html#supported-drivers
[`Pool`]: https://api.rocket.rs/v0.5-rc/rocket_db_pools/index.html#supported-drivers
[Configure]: https://api.rocket.rs/v0.5-rc/rocket_db_pools/index.html#configuration
[Derive `Database`]: https://api.rocket.rs/v0.5-rc/rocket_db_pools/derive.Database.html
[`Connection<Type>`]: https://api.rocket.rs/v0.5-rc/rocket_db_pools/struct.Connection.html
