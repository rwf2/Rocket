# Rocket Todo Example

This example makes use of a SQLite database via `diesel` to store todo tasks. As
a result, you'll need to have `sqlite3` and its headers installed:

  * **OS X:** `brew install sqlite`
  * **Debian/Ubuntu:** `apt-get install libsqlite3-dev`
  * **Arch:** `pacman -S sqlite`

## Running

**Before running, building, or testing this example, you'll need to ensure the
following:**

  1. A SQLite database file with the proper schema is present.

    On a Unix machine or with bash installed, you can simply run the
    `boostrap.sh` script to create the database. The script installs the
    `diesel_cli` tools if they're not already installed and runs the migrations.
    The script will output a `DATABASE_URL` variable.

    You can also run the migrations manually with the following commands:

    ```
    cargo install diesel_cli                      # install diesel CLI tools
    DATABASE_URL=db/db.sql diesel migration run   # create db/db.sql
    ```

  2. A `DATABASE_URL` environment variable is set that points to the SQLite
     database file.

     Use the `DATABASE_URL` variable emitted from the `bootstrap.sh` script, or
     enter it manually, as follows:

     * `DATABASE_URL=db/db.sql cargo build`
     * `DATABASE_URL=db/db.sql cargo test`
     * `DATABASE_URL=db/db.sql cargo run`
