# Rocket Databases Example

This example show you how to generate and connect to multiple connection pool.
The contributed connection_pools helper uses `r2d2`,
so in theory it can support all connection pool that can be handled by `r2d2`.

This example use 3 different connection pool, PostgreSQL using `r2d2-diesel`,
Sqlite using `r2d2_sqlite`, and Redis using `r2d2_redis`.

In order to run this application, you need to have libraries and headers for those 3:

  * **OS X:** `brew install sqlite`
  * **Debian/Ubuntu:** `apt-get install libsqlite3-dev libpq-dev redis-server`
  * **Arch:** `pacman -S sqlite redis postgresql`

## Running

**Before running, building, or testing this example, you'll need to ensure the
following:**

  1. PostgreSQL must be running and Rocket must be configured to use the proper connection.
     First go to the example directory and copy the provided `.env.example` to `.env`.
     Second modify the `.env` file to use the correct configuration to connect to your PostgreSQL instance.
     Third modify the provided `Rocket.toml` `postgres_url` to use the correct configuration to connect to your PostgreSQL instance.
     Fourth run the following commands:

     ```
     # install Diesel CLI tools
     cargo install diesel_cli

     # create db/db.sql
     diesel migration run
     ```
  2. Redis must be running and Rocket must be configured to use the proper connection.
     Modify the provided `Rocket.toml` `redis_url` to use the correct configuration to connect to your Redis server.
