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
//! ## **Table of Contents**
//!
//!   1. [Custom Attributes](#custom-attributes)
//!   2. [Custom Derives](#custom-derives)
//!       * [`FromForm`](#fromform)
//!       * [`FromFormValue`](#fromformvalue)
//!       * [`Responder`](#responder)
//!   3. [Procedural Macros](#procedural-macros)
//!   4. [Debugging Generated Code](#debugging-codegen)
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
//! This crate* implements the following custom derives:
//!
//!   * **FromForm**
//!   * **FromFormValue**
//!   * **Responder**
//!
//! <small>* In reality, all of these custom derives are currently implemented
//! by the `rocket_codegen_next` crate. Nonetheless, they are documented
//! here.</small>
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
//! Each field's type is required to implement [`FromFormValue`].
//!
//! The derive accepts one field attribute: `form`, with the following syntax:
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
//! field name is expected. In this case, the `field` name in the attribute is
//! used instead of the structure's actual field name when parsing a form.
//!
//! [`FromForm`]: /rocket/request/trait.FromForm.html
//! [`FromFormValue`]: /rocket/request/trait.FromFormValue.html
//!
//! ### `FromFormValue`
//!
//! The [`FromFormValue`] derive can be applied to enums with nullary
//! (zero-length) fields:
//!
//!     #[derive(FromFormValue)]
//!     enum MyValue {
//!         First,
//!         Second,
//!         Third,
//!     }
//!
//! The derive generates an implementation of the [`FromFormValue`] trait for
//! the decorated `enum`. The implementation returns successfully when the form
//! value matches, case insensitively, the stringified version of a variant's
//! name, returning an instance of said variant.
//!
//! As an example, for the `enum` above, the form values `"first"`, `"FIRST"`,
//! `"fiRSt"`, and so on would parse as `MyValue::First`, while `"second"` and
//! `"third"` would parse as `MyValue::Second` and `MyValue::Third`,
//! respectively.
//!
//! The `form` field attribute can be used to change the string that is compared
//! against for a given variant:
//!
//!     #[derive(FromFormValue)]
//!     enum MyValue {
//!         First,
//!         Second,
//!         #[form(value = "fourth")]
//!         Third,
//!     }
//!
//! The attribute's grammar is:
//!
//! <pre>
//! form := 'field' '=' STRING_LIT
//!
//! STRING_LIT := any valid string literal, as defined by Rust
//! </pre>
//!
//! The attribute accepts a single string parameter of name `value`
//! corresponding to the string to use to match against for the decorated
//! variant. In the example above, the the strings `"fourth"`, `"FOUrth"` and so
//! on would parse as `MyValue::Third`.
//!
//! ## `Responder`
//!
//! The [`Responder`] derive can be applied to enums and named structs. When
//! applied to enums, variants must have at least one field. When applied to
//! structs, the struct must have at least one field.
//!
//!     #[derive(Responder)]
//!     enum MyResponder {
//!         A(String),
//!         B(OtherResponse, ContentType),
//!     }
//!
//!     #[derive(Responder)]
//!     struct MyResponder {
//!         inner: OtherResponder,
//!         header: ContentType,
//!     }
//!
//! The derive generates an implementation of the [`Responder`] trait for the
//! decorated enum or structure. The derive uses the _first_ field of a variant
//! or structure to generate a `Response`. As such, the type of the first field
//! must implement [`Responder`]. The remaining fields of a variant or structure
//! are set as headers in the produced [`Response`] using
//! [`Response::set_header()`]. As such, every other field (unless explicitly
//! ignored, explained next) must implement `Into<Header>`.
//!
//! Except for the first field, fields decorated with `#[response(ignore)]` are
//! ignored by the derive:
//!
//!     #[derive(Responder)]
//!     enum MyResponder {
//!         A(String),
//!         B(OtherResponse, ContentType, #[response(ignore)] Other),
//!     }
//!
//!     #[derive(Responder)]
//!     struct MyResponder {
//!         inner: InnerResponder,
//!         header: ContentType,
//!         #[response(ignore)]
//!         other: Other,
//!     }
//!
//! Decorating the first field with `#[response(ignore)]` has no effect.
//!
//! Additionally, the `response` attribute can be used on named structures and
//! enum variants to override the status and/or content-type of the [`Response`]
//! produced by the generated implementation. The `response` attribute used in
//! these positions has the following grammar:
//!
//! <pre>
//! response := parameter (',' parameter)?
//!
//! parameter := 'status' '=' STATUS
//!            | 'content_type' '=' CONTENT_TYPE
//!
//! STATUS := unsigned integer >= 100 and < 600
//! CONTENT_TYPE := string literal, as defined by Rust, identifying a valid
//!                 Content-Type, as defined by Rocket
//! </pre>
//!
//! It can be used as follows:
//!
//!     #[derive(Responder)]
//!     enum Error {
//!         #[response(status = 500, content_type = "json")]
//!         A(String),
//!         #[response(status = 404)]
//!         B(OtherResponse, ContentType),
//!     }
//!
//!     #[derive(Responder)]
//!     #[response(status = 400)]
//!     struct MyResponder {
//!         inner: InnerResponder,
//!         header: ContentType,
//!         #[response(ignore)]
//!         other: Other,
//!     }
//!
//! The attribute accepts two key/value pairs: `status` and `content_type`. The
//! value of `status` must be an unsigned integer representing a valid status
//! code. The [`Response`] produced from the generated implementation will have
//! its status overriden to this value.
//!
//! The value of `content_type` must be a valid media-type in `top/sub` form or
//! `shorthand` form. Examples include:
//!
//!   * `"text/html"`
//!   * `"application/x-custom"`
//!   * `"html"`
//!   * `"json"`
//!   * `"plain"`
//!   * `"binary"`
//!
//! The [`Response`] produced from the generated implementation will have its
//! content-type overriden to this value.
//!
//! [`Responder`]: /rocket/response/trait.Responder.html
//! [`Response`]: /rocket/struct.Response.html
//! [`Response::set_header()`]: /rocket/struct.Response.html#method.set_header
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
//! route's URI parameters. The inputs to the macro are the path to a route, a
//! colon, and one argument for each dynamic parameter (parameters in `<>`) in
//! the route's path.
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
//! // with unnamed parameters, in route path declaration order
//! let mike = uri!(person: "Mike", 28);
//!
//! // with named parameters, order irrelevant
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
//! The `uri!` macro returns an [`Origin`](rocket::uri::Origin) structure with
//! the URI of the supplied route interpolated with the given values. Note that
//! `Origin` implements `Into<Uri>` (and by extension, `TryInto<Uri>`), so it
//! can be converted into a [`Uri`](rocket::uri::Uri) using `.into()` as needed.
//!
//!
//! A `uri!` invocation only typechecks if the type of every value in the
//! invocation matches the type declared for the parameter in the given route.
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

macro_rules! register_macros {
    ($reg:expr, $($n:expr => $f:ident),+) => (
        $($reg.register_macro($n, macros::$f);)+
    )
}

/// Compiler hook for Rust to register plugins.
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    register_macros!(reg,
        "uri" => uri,
        "rocket_internal_uri" => uri_internal
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
