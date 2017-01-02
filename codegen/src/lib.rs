//! # Rocket - Code Generation
//!
//! This crate implements the code generation portions of Rocket. This includes
//! custom derives, custom attributes, and procedural macros. The documentation
//! here is purely technical. The code generation facilities are documented
//! thoroughly in the [Rocket programming guide](https://rocket.rs/guide).
//!
//! ## Custom Attributes
//!
//! This crate implements the following custom attributes:
//!
//!   * **route**
//!   * **get**
//!   * **put**
//!   * **post**
//!   * **delete**
//!   * **head**
//!   * **patch**
//!   * **error**
//!
//! The grammar for all _route_ attributes, including **route**, **get**,
//! **put**, **post**, **delete**, **head**, and **patch**, is defined as:
//!
//! <pre>
//! route := METHOD? '(' ('path' '=')? path (',' kv_param)* ')'
//!
//! path := URI_SEG
//!       | DYNAMIC_PARAM
//!       | '?' DYNAMIC_PARAM
//!       | path '/' path
//!       (string literal)
//!
//! kv_param := 'rank' '=' INTEGER
//!           | 'format' '=' STRING
//!           | 'data' '=' DYNAMIC_PARAM
//!
//! INTEGER := isize, as defined by Rust
//! STRING := UTF-8 string literal, as defined by Rust
//! IDENT := Valid identifier, as defined by Rust
//!
//! URI_SEG := Valid HTTP URI Segment
//! DYNAMIC_PARAM := '<' IDENT '..'? '>' (string literal)
//! </pre>
//!
//! Note that the **route** attribute takes a method as its first argument,
//! while the remaining do not. That is, **route** looks like:
//!
//!     #[route(GET, path = "/hello")]
//!
//! while the equivalent using **get** looks like:
//!
//!     #[get("/hello")]
//!
//! The syntax for the **error** attribute is:
//!
//! <pre>
//! error := INTEGER
//! </pre>
//!
//! ## Custom Derives
//!
//! This crate implements the following custom derives:
//!
//!   * **FromForm**
//!
//! ## Procedural Macros
//!
//! This crate implements the following procedural macros:
//!
//!   * **routes**
//!   * **errors**
//!
//! The syntax for both of these is defined as:
//!
//! <pre>
//! macro := PATH (',' macro)*
//!
//! PATH := a path, as defined by Rust
//! </pre>
//!
//! # Debugging Codegen
//!
//! When the `ROCKET_CODEGEN_DEBUG` environment variable is set, this crate logs
//! the items it has generated to the console at compile-time. For example, you
//! might run the following to build a Rocket application with codegen logging
//! enabled:
//!
//! ```
//! ROCKET_CODEGEN_DEBUG=1 cargo build
//! ```

#![crate_type = "dylib"]
#![feature(quote, concat_idents, plugin_registrar, rustc_private, unicode)]
#![feature(custom_attribute)]
#![allow(unused_attributes)]
#![allow(deprecated)]

#[macro_use] extern crate syntax;
#[macro_use] extern crate log;
extern crate syntax_ext;
extern crate rustc;
extern crate rustc_plugin;
extern crate rocket;

#[macro_use] mod utils;
mod parser;
mod macros;
mod decorators;

use std::env;
use rustc_plugin::Registry;
use syntax::ext::base::SyntaxExtension;
use syntax::symbol::Symbol;

const DEBUG_ENV_VAR: &'static str = "ROCKET_CODEGEN_DEBUG";

const PARAM_PREFIX: &'static str = "rocket_param_";
const ROUTE_STRUCT_PREFIX: &'static str = "static_rocket_route_info_for_";
const CATCH_STRUCT_PREFIX: &'static str = "static_rocket_catch_info_for_";
const ROUTE_FN_PREFIX: &'static str = "rocket_route_fn_";
const CATCH_FN_PREFIX: &'static str = "rocket_catch_fn_";

macro_rules! register_decorators {
    ($registry:expr, $($name:expr => $func:ident),+) => (
        $($registry.register_syntax_extension(Symbol::intern($name),
                SyntaxExtension::MultiDecorator(Box::new(decorators::$func)));
         )+
    )
}

macro_rules! register_derives {
    ($registry:expr, $($name:expr => $func:ident),+) => (
        $($registry.register_custom_derive(Symbol::intern($name),
                SyntaxExtension::MultiDecorator(Box::new(decorators::$func)));
         )+
    )
}

/// Compiler hook for Rust to register plugins.
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // Enable logging early if the DEBUG_ENV_VAR is set.
    if env::var(DEBUG_ENV_VAR).is_ok() {
        ::rocket::logger::init(::rocket::LoggingLevel::Debug);
    }

    reg.register_macro("routes", macros::routes);
    reg.register_macro("errors", macros::errors);

    register_derives!(reg,
        "derive_FromForm" => from_form_derive
    );

    register_decorators!(reg,
        "error" => error_decorator,
        "route" => route_decorator,
        "get" => get_decorator,
        "put" => put_decorator,
        "post" => post_decorator,
        "delete" => delete_decorator,
        "head" => head_decorator,
        "patch" => patch_decorator
        // TODO: Allow this once Diesel incompatibility is fixed. Fix docs too.
        // "options" => options_decorator
    );
}
