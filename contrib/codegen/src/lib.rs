#![feature(proc_macro_span, proc_macro_diagnostic)]
#![recursion_limit="256"]

//! # Rocket Contrib - Code Generation
//! This crate implements the code generation portion of the Rocket Contrib
//! crate. This is for officially sanctioned contributor libraries that require
//! code generation of some kind.
//!
//! This crate includes custom derives and procedural macros and will expand
//! as-needed if future `rocket_contrib` features require code generation
//! facilities.
//!
//! ## Procedural Macros
//!
//! This crate implements the following procedural macros:
//!
//! * **databases**
//!
//! The syntax for the `databases` macro is:
//!
//! <pre>
//! macro := database(DATABASE_NAME)
//! DATABASE_NAME := (string literal)
//! </pre>

extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate quote;

mod spanned;

#[cfg(feature = "database_attribute")]
mod database;

#[allow(dead_code)]
use proc_macro::TokenStream;

#[cfg(feature = "database_attribute")]
#[proc_macro_attribute]
/// The procedural macro for the `databases` annotation.
pub fn database(attr: TokenStream, input: TokenStream) -> TokenStream {
    ::database::database_attr(attr, input).unwrap_or_else(|diag| {
        diag.emit();
        TokenStream::new()
    })
}
