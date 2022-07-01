//! Code generation for rocket-sync-db-pools.

#![recursion_limit="256"]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate quote;

mod database;

use devise::{syn, proc_macro2};
use proc_macro::TokenStream;

/// Generates a request guard and fairing for retrieving a database connection.
///
/// The syntax for the `databases` macro is:
///
/// <pre>
/// macro := 'database' '( DATABASE_NAME ')'
///
/// DATABASE_NAME := string literal
/// </pre>
///
/// The attribute accepts a single string parameter that indicates the name of
/// the database. This corresponds to the database name set as the database's
/// configuration key:
///
/// The macro generates a [`FromRequest`] implementation for the decorated type,
/// allowing the type to be used as a request guard. This implementation
/// retrieves a connection from the database pool or fails with a
/// `Status::ServiceUnavailable` if connecting to the database times out.
///
/// The macro also generates three inherent methods on the decorated type:
///
///   * `fn fairing() -> impl Fairing`
///
///      Returns a fairing that initializes the associated database connection
///      pool.
///
///   * `async fn get_one<P: Phase>(&Rocket<P>) -> Option<Self>`
///
///     Retrieves a connection wrapper from the configured pool. Returns `Some`
///     as long as `Self::fairing()` has been attached.
///
///   * `async fn run<R: Send + 'static>(&self, impl FnOnce(&mut Db) -> R + Send + 'static) -> R`
///
///     Runs the specified function or closure, providing it access to the
///     underlying database connection (`&mut Db`). Returns the value returned
///     by the function or closure.
///
/// [`FromRequest`]: rocket::request::FromRequest
#[proc_macro_attribute]
pub fn database(attr: TokenStream, input: TokenStream) -> TokenStream {
    crate::database::database_attr(attr, input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens().into())
}
