# Rocket Todo Example

This example makes use of a SQLite database via `diesel` to store todo tasks. As
a result, you'll need to have `sqlite3` and its headers installed:

  * **OS X:** `brew install sqlite`
  * **Debian/Ubuntu:** `apt-get install libsqlite3-dev`
  * **Arch:** `pacman -S sqlite`

**Before running this example, you'll also need to ensure there's a database
file with the correct tables present.** On a Unix machine or with bash
installed, you can simply run the `boostrap.sh` script. The script installs the
`diesel_cli` tools if they're not already installed and runs the migrations.

## Manually Running Migrations

You can also run the migrations manually with the following commands:

```
cargo install diesel_cli                     # install diesel CLI tools
DATABASE_URL=db/db.sql diesel migration run  # create db/db.sql
```

