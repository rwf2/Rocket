#![recursion_limit="256"]

#![warn(rust_2018_idioms)]

//! # `rocket_databases` - Code Generation
//!
//! This crate implements the code generation portion of the `rocket_databases`
//! crate.

#[macro_use] extern crate quote;

mod database;

use proc_macro::TokenStream;

/// Defines a database type and implements [`Database`] on it.
///
/// ```ignore
/// #[derive(Database)]
/// #[database(name="CONFIG_NAME")]
/// struct DBNAME(POOL_TYPE);
/// ```
///
/// `POOL_TYPE` must implement [`Pool`].
///
/// This macro generates the following code, implementing the [`Database`] trait
/// on the struct. Custom implementations of `Database` should usually also
/// start with roughly this code:
///
/// ```ignore
/// impl Database for DBNAME {
///     const NAME: &'static str = "config_name";
///     type Pool = POOL_TYPE;
///     fn fairing() -> Fairing<Self> { Fairing::new(|p| Self(p)) }
///     fn pool(&self) -> &Self::Pool { &self.0 }
/// }
/// ```
#[proc_macro_derive(Database, attributes(database))]
pub fn derive_database(input: TokenStream) -> TokenStream {
    crate::database::derive_database(input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens().into())
}
