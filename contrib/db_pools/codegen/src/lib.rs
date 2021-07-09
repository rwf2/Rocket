#![recursion_limit="256"]
#![warn(rust_2018_idioms)]

//! # `rocket_db_pool` - Code Generation
//!
//! Implements the code generation portion of the `rocket_db_pool` crate. This
//! is an implementation detail. This create should never be depended on
//! directly.

#[macro_use] extern crate quote;

mod database;

/// Automatic derive for the [`Database`] trait.
///
/// ```rust
/// use rocket_db_pools::Database;
/// # type PoolType = rocket_db_pools::deadpool_postgres::Pool;
///
/// #[derive(Database)]
/// #[database("database_name")]
/// struct Db(PoolType);
/// ```
///
/// The derive generates an implementation of [`Database`] as follows:
///
/// * [`Database::NAME`] is set to the value in the `#[database("name")]`
///   attribute.
///
///   This names the database, providing an anchor to configure the database via
///   `Rocket.toml` or any other configuration source. Specifically, the
///   configuration in `databases.name` is used to configure the driver.
///
/// * [`Database::Pool`] is set to the wrapped type: `PoolType` above. The type
///   must implement [`Pool`].
///
/// To meet the required [`Database`] supertrait bounds, this derive also
/// generates implementations for:
///
/// * `From<Db::Pool>`
///
/// * `Deref<Target = Db::Pool>`
///
/// * `DerefMut<Target = Db::Pool>`
///
/// * `FromRequest<'_> for &Db`
///
/// * `Sentinel for &Db`
///
/// The `Deref` impls enable accessing the database pool directly from
/// references `&Db` or `&mut Db`. To force a dereference to the underlying
/// type, use `&db.0` or `&**db` or their `&mut` variants.
///
/// [`Database`]: ../rocket_db_pools/trait.Database.html
/// [`Database::NAME`]: ../rocket_db_pools/trait.Database.html#associatedconstant.NAME
/// [`Database::Pool`]: ../rocket_db_pools/trait.Database.html#associatedtype.Pool
/// [`Pool`]: ../rocket_db_pools/trait.Pool.html
#[proc_macro_derive(Database, attributes(database))]
pub fn derive_database(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    crate::database::derive_database(input)
}
