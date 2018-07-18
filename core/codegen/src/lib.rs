#![crate_type = "dylib"]
#![feature(quote, concat_idents, plugin_registrar, rustc_private)]
#![feature(custom_attribute)]
#![allow(unused_attributes)]
#![allow(deprecated)]

// TODO: Version URLs.
#![doc(html_root_url = "https://api.rocket.rs")]

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
//!   * **options**
//!   * **catch**
//!
//! The grammar for all _route_ attributes, including **route**, **get**,
//! **put**, **post**, **delete**, **head**, **patch**, and **options** is
//! defined as:
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
//! IDENT := valid identifier, as defined by Rust
//!
//! URI_SEG := valid HTTP URI Segment
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
//! The syntax for the **catch** attribute is:
//!
//! <pre>
//! catch := INTEGER
//! </pre>
//!
//! A use of the `catch` attribute looks like:
//!
//!     #[catch(404)]
//!
//! ## Custom Derives
//!
//! This crate implements the following custom derives:
//!
//!   * **FromForm**
//!
//! ### `FromForm`
//!
//! The [`FromForm`] derive can be applied to structures with named fields:
//!
//!     #[derive(FromForm)]
//!     struct MyStruct {
//!         field: usize,
//!         other: String
//!     }
//!
//! Each field's type is required to implement [`FromFormValue`]. The derive
//! accepts one field attribute: `form`, with the following syntax:
//!
//! <pre>
//! form := 'field' '=' '"' IDENT '"'
//!
//! IDENT := valid identifier, as defined by Rust
//! </pre>
//!
//! When applied, the attribute looks as follows:
//!
//!     #[derive(FromForm)]
//!     struct MyStruct {
//!         field: usize,
//!         #[form(field = "renamed_field")]
//!         other: String
//!     }
//!
//! The derive generates an implementation for the [`FromForm`] trait. The
//! implementation parses a form whose field names match the field names of the
//! structure on which the derive was applied. Each field's value is parsed with
//! the [`FromFormValue`] implementation of the field's type. The `FromForm`
//! implementation succeeds only when all of the field parses succeed.
//!
//! The `form` field attribute can be used to direct that a different incoming
//! field name is expected. In this case, the attribute's field name is used
//! instead of the structure's field name when parsing a form.
//!
//! [`FromForm`]: /rocket/request/trait.FromForm.html
//! [`FromFormValue`]: /rocket/request/trait.FromFormValue.html
//!
//! ## Procedural Macros
//!
//! This crate implements the following procedural macros:
//!
//!   * **routes**
//!   * **catchers**
//!   * **uri**
//!
//! The syntax for `routes!` and `catchers!` is defined as:
//!
//! <pre>
//! macro := PATH (',' PATH)*
//!
//! PATH := a path, as defined by Rust
//! </pre>
//!
//! ### Typed URIs: `uri!`
//!
//! The `uri!` macro creates a type-safe URI given a route and values for the
//! route's URI parameters.
//!
//! For example, for the following route:
//!
//! ```rust,ignore
//! #[get("/person/<name>/<age>")]
//! fn person(name: String, age: u8) -> String {
//!     format!("Hello {}! You're {} years old.", name, age)
//! }
//! ```
//!
//! A URI can be created as follows:
//!
//! ```rust,ignore
//! // with unnamed parameters
//! let mike = uri!(person: "Mike", 28);
//!
//! // with named parameters
//! let mike = uri!(person: name = "Mike", age = 28);
//! let mike = uri!(person: age = 28, name = "Mike");
//!
//! // with a specific mount-point
//! let mike = uri!("/api", person: name = "Mike", age = 28);
//! ```
//!
//! #### Grammar
//!
//! The grammar for the `uri!` macro is as follows:
//!
//! <pre>
//! uri := (mount ',')? PATH (':' params)?
//!
//! mount = STRING
//! params := unnamed | named
//! unnamed := EXPR (',' EXPR)*
//! named := IDENT = EXPR (',' named)?
//!
//! EXPR := a valid Rust expression (examples: `foo()`, `12`, `"hey"`)
//! IDENT := a valid Rust identifier (examples: `name`, `age`)
//! STRING := an uncooked string literal, as defined by Rust (example: `"hi"`)
//! PATH := a path, as defined by Rust (examples: `route`, `my_mod::route`)
//! </pre>
//!
//! #### Semantics
//!
//! The `uri!` macro returns a `Uri` structure with the URI of the supplied
//! route with the given values. A `uri!` invocation only succeeds if the type
//! of every value in the invocation matches the type declared for the parameter
//! in the given route.
//!
//! The [`FromUriParam`] trait is used to typecheck and perform a conversion for
//! each value. If a `FromUriParam<S>` implementation exists for a type `T`,
//! then a value of type `S` can be used in `uri!` macro for a route URI
//! parameter declared with a type of `T`. For example, the following
//! implementation, provided by Rocket, allows an `&str` to be used in a `uri!`
//! invocation for route URI parameters declared as `String`:
//!
//! ```
//! impl<'a> FromUriParam<&'a str> for String
//! ```
//!
//! Each value passed into `uri!` is rendered in its appropriate place in the
//! URI using the [`UriDisplay`] implementation for the value's type. The
//! `UriDisplay` implementation ensures that the rendered value is URI-safe.
//!
//! If a mount-point is provided, the mount-point is prepended to the route's
//! URI.
//!
//! [`Uri`]: /rocket/http/uri/struct.URI.html
//! [`FromUriParam`]: /rocket/http/uri/trait.FromUriParam.html
//! [`UriDisplay`]: /rocket/http/uri/trait.UriDisplay.html
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


extern crate syntax;
extern crate syntax_ext;
extern crate syntax_pos;
extern crate rustc_plugin;
extern crate rocket_http;
extern crate indexmap;

#[macro_use] mod utils;
mod parser;
mod macros;
mod decorators;

use rustc_plugin::Registry;
use syntax::ext::base::SyntaxExtension;
use syntax::symbol::Symbol;

const DEBUG_ENV_VAR: &str = "ROCKET_CODEGEN_DEBUG";

const PARAM_PREFIX: &str = "rocket_param_";
const ROUTE_STRUCT_PREFIX: &str = "static_rocket_route_info_for_";
const CATCH_STRUCT_PREFIX: &str = "static_rocket_catch_info_for_";
const ROUTE_FN_PREFIX: &str = "rocket_route_fn_";
const CATCH_FN_PREFIX: &str = "rocket_catch_fn_";
const URI_INFO_MACRO_PREFIX: &str = "rocket_uri_for_";

const ROUTE_ATTR: &str = "rocket_route";
const ROUTE_INFO_ATTR: &str = "rocket_route_info";

const CATCHER_ATTR: &str = "rocket_catcher";

macro_rules! register_decorators {
    ($registry:expr, $($name:expr => $func:ident),+) => (
        $($registry.register_syntax_extension(Symbol::intern($name),
                SyntaxExtension::MultiModifier(Box::new(decorators::$func)));
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

macro_rules! register_macros {
    ($reg:expr, $($n:expr => $f:ident),+) => (
        $($reg.register_macro($n, macros::$f);)+
    )
}

/// Compiler hook for Rust to register plugins.
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    register_macros!(reg,
        "routes" => routes,
        "catchers" => catchers,
        "uri" => uri,
        "rocket_internal_uri" => uri_internal
    );

    register_derives!(reg,
        "derive_FromForm" => from_form_derive
    );

    register_decorators!(reg,
        "catch" => catch_decorator,
        "route" => route_decorator,
        "get" => get_decorator,
        "put" => put_decorator,
        "post" => post_decorator,
        "delete" => delete_decorator,
        "head" => head_decorator,
        "patch" => patch_decorator,
        "options" => options_decorator
    );
}
