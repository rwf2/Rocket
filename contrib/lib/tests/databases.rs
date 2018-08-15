#![feature(use_extern_macros)]

extern crate rocket;
extern crate rocket_contrib;

#[cfg(feature = "databases")]
mod databases_tests {
    use rocket_contrib::databases::{database, diesel};

    #[database("foo")]
    struct TempStorage(diesel::SqliteConnection);

    #[database("bar")]
    struct PrimaryDb(diesel::PgConnection);
}
