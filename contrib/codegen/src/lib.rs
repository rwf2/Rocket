#![feature(proc_macro_span, proc_macro_diagnostic)]
#![feature(crate_visibility_modifier)]
#![recursion_limit="256"]

#![warn(rust_2018_idioms)]

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

extern crate proc_macro;

#[allow(unused_imports)]
#[macro_use] extern crate quote;

#[allow(unused_imports)]
crate use devise::{syn, proc_macro2};

#[cfg(feature = "database_attribute")]
mod database;

#[allow(unused_imports)]
use proc_macro::TokenStream;

/// The procedural macro for the `databases` annotation.
#[cfg(feature = "database_attribute")]
#[proc_macro_attribute]
pub fn database(attr: TokenStream, input: TokenStream) -> TokenStream {
    crate::database::database_attr(attr, input).unwrap_or_else(|diag| {
        diag.emit();
        TokenStream::new()
    })
}
