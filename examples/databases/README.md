# Databases Example

This example makes use of SQLite and MySQL. You'll need `sqlite3` and a MySQL
client installed:

  * **macOS:** `brew install sqlite mysql-client`
  * **Debian**, **Ubuntu:** `apt-get install libsqlite3-dev libmysqlclient-dev`
  * **Arch:** `pacman -S sqlite libmysqlclient`

## API Implementation

This example implements a JSON-based HTTP API for a "blog" using several database drivers:

  * `sqlx` (`/sqlx`, `sqlx.rs`)
  * `rusqlite` (`/rusqlite`, `rusqlite.rs`)
  * `diesel` (sqlite) (`/diesel`, `diesel_sqlite.rs`)
  * `diesel-async` (mysql) (`/mysql`, `diesel_mysql.rs`)

The exposed API is succinctly described as follows, with
[`httpie`](https://httpie.io/) CLI examples:

  * `POST /driver`: create post via JSON with `title` and `text`; returns new
    post JSON with new `id`

        http http://127.0.0.1:8000/sqlx title="Title" text="Hello, world."
        > { "id": 2128, "text": "Hello, world.", "title": "Title" }

  * `GET /driver`: returns JSON array of IDs for blog posts

        http http://127.0.0.1:8000/sqlx
        > [ 2128, 2129, 2130, 2131 ]

  * `GET /driver/<id>`: returns a JSON object for the post with id `<id>`

        http http://127.0.0.1:8000/sqlx/2128
        > { "id": 2128, "text": "Hello, world.", "title": "Title" }

  * `DELETE /driver`: delete all posts

        http delete http://127.0.0.1:8000/sqlx

  * `DELETE /driver/<id>`: delete post with id `<id>`

        http delete http://127.0.0.1:8000/sqlx/4

## Migrations

Database migrations are stored in the respective `db/${driver}` directory.

### `diesel`

Diesel migrations are found in `db/diesel/migrations`. They are run
automatically. They can be run manually as well:

```sh
cargo install diesel_cli --no-default-features --features sqlite
DATABASE_URL="db/diesel/db.sqlite" diesel migration --migration-dir db/diesel/migrations redo
```

### `sqlx`

sqlx migrations are found in `db/sqlx/migrations`. They are run automatically.

Query metadata for offline checking was prepared with the following commands:

```sh
cargo install sqlx-cli --no-default-features --features sqlite
DATABASE_URL="sqlite:$(pwd)/db/sqlx/db.sqlite" cargo sqlx prepare
```
