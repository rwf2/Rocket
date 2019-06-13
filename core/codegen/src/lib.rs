#![feature(proc_macro_diagnostic, proc_macro_span)]
#![feature(crate_visibility_modifier)]
#![recursion_limit="128"]

#![doc(html_root_url = "https://api.rocket.rs/v0.5")]
#![doc(html_favicon_url = "https://rocket.rs/v0.5/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/v0.5/images/logo-boxed.png")]

#![warn(rust_2018_idioms)]

//! # Rocket - Code Generation
//!
//! This crate implements the code generation portions of Rocket. This includes
//! custom derives, custom attributes, and procedural macros. The documentation
//! here is purely technical. The code generation facilities are documented
//! thoroughly in the [Rocket programming guide](https://rocket.rs/v0.5/guide).
//!
//! # Usage
//!
//! You **_should not_** directly depend on this library. To use the macros,
//! attributes, and derives in this crate, it suffices to depend on `rocket` in
//! `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = "0.5.0-dev"
//! ```
//!
//! And to import all macros, attributes, and derives via `#[macro_use]` in the
//! crate root:
//!
//! ```rust
//! #![feature(proc_macro_hygiene, decl_macro)]
//!
//! #[macro_use] extern crate rocket;
//! # #[get("/")] fn hello() { }
//! # fn main() { rocket::ignite().mount("/", routes![hello]); }
//! ```
//!
//! Or, alternatively, selectively import from the top-level scope:
//!
//! ```rust
//! #![feature(proc_macro_hygiene, decl_macro)]
//! # extern crate rocket;
//!
//! use rocket::{get, routes};
//! # #[get("/")] fn hello() { }
//! # fn main() { rocket::ignite().mount("/", routes![hello]); }
//! ```
//!
//! # Debugging Codegen
//!
//! When the `ROCKET_CODEGEN_DEBUG` environment variable is set, this crate
//! logs, at compile-time and to the console, the items it generates. For
//! example, you might run the following to build a Rocket application with
//! codegen debug logging enabled:
//!
//! ```sh
//! ROCKET_CODEGEN_DEBUG=1 cargo build
//! ```

#[macro_use] extern crate quote;
extern crate proc_macro;

use rocket_http as http;

macro_rules! define {
    ($val:path as $v:ident) => (#[allow(non_snake_case)] let $v = quote!($val););
}

macro_rules! define_vars_and_mods {
    (@req as $v:ident) => (define!(__req as $v));
    (@catcher as $v:ident) => (define!(__catcher as $v));
    (@data as $v:ident) => (define!(__data as $v));
    (@error as $v:ident) => (define!(__error as $v));
    (@trail as $v:ident) => (define!(__trail as $v));
    (@request as $v:ident) => (define!(::rocket::request as $v));
    (@response as $v:ident) => (define!(::rocket::response as $v));
    (@handler as $v:ident) => (define!(::rocket::handler as $v));
    (@log as $v:ident) => (define!(::rocket::logger as $v));
    (@Outcome as $v:ident) => (define!(::rocket::Outcome as $v));
    (@FromData as $v:ident) => (define!(::rocket::data::FromData as $v));
    (@Transform as $v:ident) => (define!(::rocket::data::Transform as $v));
    (@Query as $v:ident) => (define!(::rocket::request::Query as $v));
    (@Request as $v:ident) => (define!(::rocket::Request as $v));
    (@Response as $v:ident) => (define!(::rocket::response::Response as $v));
    (@Data as $v:ident) => (define!(::rocket::Data as $v));
    (@StaticRouteInfo as $v:ident) => (define!(::rocket::StaticRouteInfo as $v));
    (@SmallVec as $v:ident) => (define!(::rocket::http::private::SmallVec as $v));
    ($($name:ident),*) => ($(define_vars_and_mods!(@$name as $name);)*)
}

#[macro_use]
mod proc_macro_ext;
mod derive;
mod attribute;
mod bang;
mod http_codegen;
mod syn_ext;

use crate::http::Method;
use proc_macro::TokenStream;
crate use devise::proc_macro2;

crate static ROUTE_STRUCT_PREFIX: &str = "static_rocket_route_info_for_";
crate static CATCH_STRUCT_PREFIX: &str = "static_rocket_catch_info_for_";
crate static CATCH_FN_PREFIX: &str = "rocket_catch_fn_";
crate static ROUTE_FN_PREFIX: &str = "rocket_route_fn_";
crate static URI_MACRO_PREFIX: &str = "rocket_uri_macro_";
crate static ROCKET_PARAM_PREFIX: &str = "__rocket_param_";

macro_rules! emit {
    ($tokens:expr) => ({
        let tokens = $tokens;
        if std::env::var_os("ROCKET_CODEGEN_DEBUG").is_some() {
            proc_macro::Span::call_site()
                .note("emitting Rocket code generation debug output")
                .note(tokens.to_string())
                .emit()
        }

        tokens
    })
}

macro_rules! route_attribute {
    ($name:ident => $method:expr) => (
        /// Attribute to generate a [`Route`] and associated metadata.
        ///
        /// This and all other route attributes can only be applied to free
        /// functions:
        ///
        /// ```rust
        /// # #![feature(proc_macro_hygiene, decl_macro)]
        /// # #[macro_use] extern crate rocket;
        /// #
        /// #[get("/")]
        /// fn index() -> &'static str {
        ///     "Hello, world!"
        /// }
        /// ```
        ///
        /// There are 7 method-specific route attributes:
        ///
        ///   * [`#[get]`] - `GET` specific route
        ///   * [`#[put]`] - `PUT` specific route
        ///   * [`#[post]`] - `POST` specific route
        ///   * [`#[delete]`] - `DELETE` specific route
        ///   * [`#[head]`] - `HEAD` specific route
        ///   * [`#[options]`] - `OPTIONS` specific route
        ///   * [`#[patch]`] - `PATCH` specific route
        ///
        /// Additionally, [`#[route]`] allows the method and path to be
        /// explicitly specified:
        ///
        /// ```rust
        /// # #![feature(proc_macro_hygiene, decl_macro)]
        /// # #[macro_use] extern crate rocket;
        /// #
        /// #[route(GET, path = "/")]
        /// fn index() -> &'static str {
        ///     "Hello, world!"
        /// }
        /// ```
        ///
        /// [`#[delete]`]: attr.delete.html
        /// [`#[get]`]: attr.get.html
        /// [`#[head]`]: attr.head.html
        /// [`#[options]`]: attr.options.html
        /// [`#[patch]`]: attr.patch.html
        /// [`#[post]`]: attr.post.html
        /// [`#[put]`]: attr.put.html
        /// [`#[route]`]: attr.route.html
        ///
        /// # Grammar
        ///
        /// The grammar for all method-specific route attributes is defined as:
        ///
        /// ```text
        /// route := '"' path ('?' query)? '"' (',' parameter)*
        ///
        /// path := ('/' segment)*
        ///
        /// query := segment ('&' segment)*
        ///
        /// segment := URI_SEG
        ///          | SINGLE_PARAM
        ///          | MULTI_PARAM
        ///
        /// parameter := 'rank' '=' INTEGER
        ///            | 'format' '=' '"' MEDIA_TYPE '"'
        ///            | 'data' '=' '"' SINGLE_PARAM '"'
        ///
        /// SINGLE_PARAM := '<' IDENT '>'
        /// MULTI_PARAM := '<' IDENT '..>'
        ///
        /// URI_SEG := valid, non-percent-encoded HTTP URI segment
        /// MEDIA_TYPE := valid HTTP media type or known shorthand
        ///
        /// INTEGER := unsigned integer, as defined by Rust
        /// IDENT := valid identifier, as defined by Rust, except `_`
        /// ```
        ///
        /// The generic route attribute is defined as:
        ///
        /// ```text
        /// generic-route := METHOD ',' 'path' '=' route
        /// ```
        ///
        /// # Typing Requirements
        ///
        /// Every identifier that appears in a dynamic parameter (`SINGLE_PARAM`
        /// or `MULTI_PARAM`) must appear as an argument to the function. For
        /// example, the following route requires the decorated function to have
        /// the arguments `foo`, `baz`, `msg`, `rest`, and `form`:
        ///
        /// ```rust
        /// # #![feature(proc_macro_hygiene, decl_macro)]
        /// # #[macro_use] extern crate rocket;
        /// # use rocket::request::Form;
        /// # use std::path::PathBuf;
        /// # #[derive(FromForm)] struct F { a: usize }
        /// #[get("/<foo>/bar/<baz..>?<msg>&closed&<rest..>", data = "<form>")]
        /// # fn f(foo: usize, baz: PathBuf, msg: String, rest: Form<F>, form: Form<F>) {  }
        /// ```
        ///
        /// The type of each function argument corresponding to a dynamic
        /// parameter is required to implement one of Rocket's guard traits. The
        /// exact trait that is required to be implemented depends on the kind
        /// of dynamic parameter (`SINGLE` or `MULTI`) and where in the route
        /// attribute the parameter appears. The table below summarizes trait
        /// requirements:
        ///
        /// | position | kind        | trait             |
        /// |----------|-------------|-------------------|
        /// | path     | `<ident>`   | [`FromParam`]     |
        /// | path     | `<ident..>` | [`FromSegments`]  |
        /// | query    | `<ident>`   | [`FromFormValue`] |
        /// | query    | `<ident..>` | [`FromQuery`]     |
        /// | data     | `<ident>`   | [`FromData`]      |
        ///
        /// The type of each function argument that _does not_ have a
        /// corresponding dynamic parameter is required to implement the
        /// [`FromRequest`] trait.
        ///
        /// The return type of the decorated function must implement the
        /// [`Responder`] trait.
        ///
        /// [`FromParam`]: ../rocket/request/trait.FromParam.html
        /// [`FromSegments`]: ../rocket/request/trait.FromSegments.html
        /// [`FromFormValue`]: ../rocket/request/form/trait.FromFormValue.html
        /// [`FromQuery`]: ../rocket/request/trait.FromQuery.html
        /// [`FromData`]: ../rocket/data/trait.FromData.html
        /// [`FromRequest`]: ../rocket/request/trait.FromRequest.html
        /// [`Route`]: ../rocket/struct.Route.html
        /// [`Responder`]: ../rocket/response/trait.Responder.html
        ///
        /// # Semantics
        ///
        /// The attribute generates three items:
        ///
        ///   1. A route [`Handler`].
        ///
        ///      The generated handler validates and generates all arguments for
        ///      the generated function according to the trait that their type
        ///      must implement. The order in which arguments are processed is:
        ///
        ///         1. Request guards from left to right.
        ///
        ///            If a request guard fails, the request is forwarded if the
        ///            [`Outcome`] is `Forward` or failed if the [`Outcome`] is
        ///            `Failure`. See [`FromRequest` Outcomes] for further
        ///            detail.
        ///
        ///         2. Path and query parameters from left to right as declared
        ///            in the function argument list.
        ///
        ///            If a path or query parameter guard fails, the request is
        ///            forwarded.
        ///
        ///         3. Data parameter, if any.
        ///
        ///            If a data guard fails, the request is forwarded if the
        ///            [`Outcome`] is `Forward` or failed if the [`Outcome`] is
        ///            `Failure`. See [`FromData` Outcomes] for further detail.
        ///
        ///      If all validation succeeds, the decorated function is called.
        ///      The returned value is used to generate a [`Response`] via the
        ///      type's [`Responder`] implementation.
        ///
        ///   2. A static structure used by [`routes!`] to generate a [`Route`].
        ///
        ///      The static structure (and resulting [`Route`]) is populated
        ///      with the name (the function's name), path, query, rank, and
        ///      format from the route attribute. The handler is set to the
        ///      generated handler.
        ///
        ///   3. A macro used by [`uri!`] to type-check and generate an
        ///      [`Origin`].
        ///
        /// [`Handler`]: ../rocket/trait.Handler.html
        /// [`routes!`]: macro.routes.html
        /// [`uri!`]: macro.uri.html
        /// [`Origin`]: ../rocket/http/uri/struct.Origin.html
        /// [`Outcome`]: ../rocket/enum.Outcome.html
        /// [`Response`]: ../rocket/struct.Response.html
        /// [`FromRequest` Outcomes]: ../rocket/request/trait.FromRequest.html#outcomes
        /// [`FromData` Outcomes]: ../rocket/data/trait.FromData.html#outcomes
        #[proc_macro_attribute]
        pub fn $name(args: TokenStream, input: TokenStream) -> TokenStream {
            emit!(attribute::route::route_attribute($method, args, input))
        }
    )
}

route_attribute!(route => None);
route_attribute!(get => Method::Get);
route_attribute!(put => Method::Put);
route_attribute!(post => Method::Post);
route_attribute!(delete => Method::Delete);
route_attribute!(head => Method::Head);
route_attribute!(patch => Method::Patch);
route_attribute!(options => Method::Options);

/// Attribute to generate a [`Catcher`] and associated metadata.
///
/// This attribute can only be applied to free functions:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::Request;
///
/// #[catch(404)]
/// fn not_found(req: &Request) -> String {
///     format!("Sorry, {} does not exist.", req.uri())
/// }
/// ```
///
/// # Grammar
///
/// The grammar for the `#[catch]` attributes is defined as:
///
/// ```text
/// catch := STATUS
///
/// STATUS := valid HTTP status code (integer in [200, 599])
/// ```
///
/// # Typing Requirements
///
/// The decorated function must take exactly zero or one argument. If the
/// decorated function takes an argument, the argument's type must be
/// [`&Request`].
///
/// The return type of the decorated function must implement the [`Responder`]
/// trait.
///
/// # Semantics
///
/// The attribute generates two items:
///
///   1. An [`ErrorHandler`].
///
///      The generated handler calls the decorated function, passing in the
///      [`&Request`] value if requested. The returned value is used to generate
///      a [`Response`] via the type's [`Responder`] implementation.
///
///   2. A static structure used by [`catchers!`] to generate a [`Catcher`].
///
///      The static structure (and resulting [`Catcher`]) is populated
///      with the name (the function's name) and status code from the
///      route attribute. The handler is set to the generated handler.
///
/// [`&Request`]: ../rocket/struct.Request.html
/// [`ErrorHandler`]: ../rocket/type.ErrorHandler.html
/// [`catchers!`]: macro.catchers.html
/// [`Catcher`]: ../rocket/struct.Catcher.html
/// [`Response`]: ../rocket/struct.Response.html
/// [`Responder`]: ../rocket/response/trait.Responder.html
#[proc_macro_attribute]
pub fn catch(args: TokenStream, input: TokenStream) -> TokenStream {
    emit!(attribute::catch::catch_attribute(args, input))
}

/// Derive for the [`FromFormValue`] trait.
///
/// The [`FromFormValue`] derive can be applied to enums with nullary
/// (zero-length) fields:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// #[derive(FromFormValue)]
/// enum MyValue {
///     First,
///     Second,
///     Third,
/// }
/// ```
///
/// The derive generates an implementation of the [`FromFormValue`] trait for
/// the decorated `enum`. The implementation returns successfully when the form
/// value matches, case insensitively, the stringified version of a variant's
/// name, returning an instance of said variant. If there is no match, an error
/// ([`FromFormValue::Error`]) of type [`&RawStr`] is returned, the value of
/// which is the raw form field value that failed to match.
///
/// As an example, for the `enum` above, the form values `"first"`, `"FIRST"`,
/// `"fiRSt"`, and so on would parse as `MyValue::First`, while `"second"` and
/// `"third"` would parse as `MyValue::Second` and `MyValue::Third`,
/// respectively.
///
/// The `form` field attribute can be used to change the string that is compared
/// against for a given variant:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// #[derive(FromFormValue)]
/// enum MyValue {
///     First,
///     Second,
///     #[form(value = "fourth")]
///     Third,
/// }
/// ```
///
/// The `#[form]` attribute's grammar is:
///
/// ```text
/// form := 'field' '=' STRING_LIT
///
/// STRING_LIT := any valid string literal, as defined by Rust
/// ```
///
/// The attribute accepts a single string parameter of name `value`
/// corresponding to the string to use to match against for the decorated
/// variant. In the example above, the the strings `"fourth"`, `"FOUrth"` and so
/// on would parse as `MyValue::Third`.
///
/// [`FromFormValue`]: ../rocket/request/trait.FromFormValue.html
/// [`FromFormValue::Error`]: ../rocket/request/trait.FromFormValue.html#associatedtype.Error
/// [`&RawStr`]: ../rocket/http/struct.RawStr.html
// FIXME(rustdoc): We should be able to refer to items in `rocket`.
#[proc_macro_derive(FromFormValue, attributes(form))]
pub fn derive_from_form_value(input: TokenStream) -> TokenStream {
    emit!(derive::from_form_value::derive_from_form_value(input))
}

/// Derive for the [`FromForm`] trait.
///
/// The [`FromForm`] derive can be applied to structures with named fields:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// #[derive(FromForm)]
/// struct MyStruct {
///     field: usize,
///     other: String
/// }
/// ```
///
/// Each field's type is required to implement [`FromFormValue`].
///
/// The derive generates an implementation of the [`FromForm`] trait. The
/// implementation parses a form whose field names match the field names of the
/// structure on which the derive was applied. Each field's value is parsed with
/// the [`FromFormValue`] implementation of the field's type. The `FromForm`
/// implementation succeeds only when all of the field parses succeed. If
/// parsing fails, an error ([`FromForm::Error`]) of type [`FormParseError`] is
/// returned.
///
/// The derive accepts one field attribute: `form`, with the following syntax:
///
/// ```text
/// form := 'field' '=' '"' IDENT '"'
///
/// IDENT := valid identifier, as defined by Rust
/// ```
///
/// When applied, the attribute looks as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// #[derive(FromForm)]
/// struct MyStruct {
///     field: usize,
///     #[form(field = "renamed_field")]
///     other: String
/// }
/// ```
///
/// The field attribute directs that a different incoming field name is
/// expected, and the value of the `field` attribute is used instead of the
/// structure's actual field name when parsing a form. In the example above, the
/// value of the `MyStruct::other` struct field will be parsed from the incoming
/// form's `renamed_field` field.
///
/// [`FromForm`]: ../rocket/request/trait.FromForm.html
/// [`FromFormValue`]: ../rocket/request/trait.FromFormValue.html
/// [`FormParseError`]: ../rocket/request/enum.FormParseError.html
/// [`FromForm::Error`]: ../rocket/request/trait.FromForm.html#associatedtype.Error
#[proc_macro_derive(FromForm, attributes(form))]
pub fn derive_from_form(input: TokenStream) -> TokenStream {
    emit!(derive::from_form::derive_from_form(input))
}

/// Derive for the [`Responder`] trait.
///
/// The [`Responder`] derive can be applied to enums and structs with named
/// fields. When applied to enums, variants must have at least one field. When
/// applied to structs, the struct must have at least one field.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use std::fs::File;
/// # use rocket::http::ContentType;
/// # type OtherResponder = MyResponderA;
/// #
/// #[derive(Responder)]
/// enum MyResponderA {
///     A(String),
///     B(File, ContentType),
/// }
///
/// #[derive(Responder)]
/// struct MyResponderB {
///     inner: OtherResponder,
///     header: ContentType,
/// }
/// ```
///
/// The derive generates an implementation of the [`Responder`] trait for the
/// decorated enum or structure. The derive uses the _first_ field of a variant
/// or structure to generate a [`Response`]. As such, the type of the first
/// field must implement [`Responder`]. The remaining fields of a variant or
/// structure are set as headers in the produced [`Response`] using
/// [`Response::set_header()`]. As such, every other field (unless explicitly
/// ignored, explained next) must implement `Into<Header>`.
///
/// Except for the first field, fields decorated with `#[response(ignore)]` are
/// ignored by the derive:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use std::fs::File;
/// # use rocket::http::ContentType;
/// # use rocket::response::NamedFile;
/// # type Other = usize;
/// #
/// #[derive(Responder)]
/// enum MyResponder {
///     A(String),
///     B(File, ContentType, #[response(ignore)] Other),
/// }
///
/// #[derive(Responder)]
/// struct MyOtherResponder {
///     inner: NamedFile,
///     header: ContentType,
///     #[response(ignore)]
///     other: Other,
/// }
/// ```
///
/// Decorating the first field with `#[response(ignore)]` has no effect.
///
/// Additionally, the `response` attribute can be used on named structures and
/// enum variants to override the status and/or content-type of the [`Response`]
/// produced by the generated implementation. The `response` attribute used in
/// these positions has the following grammar:
///
/// ```text
/// response := parameter (',' parameter)?
///
/// parameter := 'status' '=' STATUS
///            | 'content_type' '=' CONTENT_TYPE
///
/// STATUS := unsigned integer >= 100 and < 600
/// CONTENT_TYPE := string literal, as defined by Rust, identifying a valid
///                 Content-Type, as defined by Rocket
/// ```
///
/// It can be used as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket::http::ContentType;
/// # use rocket::response::NamedFile;
/// # type Other = usize;
/// # type InnerResponder = String;
/// #
/// #[derive(Responder)]
/// enum Error {
///     #[response(status = 500, content_type = "json")]
///     A(String),
///     #[response(status = 404)]
///     B(NamedFile, ContentType),
/// }
///
/// #[derive(Responder)]
/// #[response(status = 400)]
/// struct MyResponder {
///     inner: InnerResponder,
///     header: ContentType,
///     #[response(ignore)]
///     other: Other,
/// }
/// ```
///
/// The attribute accepts two key/value pairs: `status` and `content_type`. The
/// value of `status` must be an unsigned integer representing a valid status
/// code. The [`Response`] produced from the generated implementation will have
/// its status overriden to this value.
///
/// The value of `content_type` must be a valid media-type in `top/sub` form or
/// `shorthand` form. Examples include:
///
///   * `"text/html"`
///   * `"application/x-custom"`
///   * `"html"`
///   * `"json"`
///   * `"plain"`
///   * `"binary"`
///
/// See [`ContentType::parse_flexible()`] for a full list of available
/// shorthands. The [`Response`] produced from the generated implementation will
/// have its content-type overriden to this value.
///
/// [`Responder`]: ../rocket/response/trait.Responder.html
/// [`Response`]: ../rocket/struct.Response.html
/// [`Response::set_header()`]: ../rocket/response/struct.Response.html#method.set_header
/// [`ContentType::parse_flexible()`]: ../rocket/http/struct.ContentType.html#method.parse_flexible
#[proc_macro_derive(Responder, attributes(response))]
pub fn derive_responder(input: TokenStream) -> TokenStream {
    emit!(derive::responder::derive_responder(input))
}

/// Derive for the [`UriDisplay<Query>`] trait.
///
/// The [`UriDisplay<Query>`] derive can be applied to enums and structs. When
/// applied to enums, variants must have at least one field. When applied to
/// structs, the struct must have at least one field.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[derive(UriDisplayQuery)]
/// enum Kind {
///     A(String),
///     B(usize),
/// }
///
/// #[derive(UriDisplayQuery)]
/// struct MyStruct {
///     name: String,
///     id: usize,
///     kind: Kind,
/// }
/// ```
///
/// Each field's type is required to implement [`UriDisplay<Query>`].
///
/// The derive generates an implementation of the [`UriDisplay<Query>`] trait.
/// The implementation calls [`Formatter::write_named_value()`] for every named
/// field, using the field's name (unless overriden, explained next) as the
/// `name` parameter, and [`Formatter::write_value()`] for every unnamed field
/// in the order the fields are declared.
///
/// The derive accepts one field attribute: `form`, with the following syntax:
///
/// ```text
/// form := 'field' '=' '"' IDENT '"'
///
/// IDENT := valid identifier, as defined by Rust
/// ```
///
/// When applied, the attribute looks as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[derive(UriDisplayQuery)]
/// # struct Kind(String);
/// #[derive(UriDisplayQuery)]
/// struct MyStruct {
///     name: String,
///     id: usize,
///     #[form(field = "type")]
///     kind: Kind,
/// }
/// ```
///
/// The field attribute directs that a different field name be used when calling
/// [`Formatter::write_named_value()`] for the given field. The value of the
/// `field` attribute is used instead of the structure's actual field name. In
/// the example above, the field `MyStruct::kind` is rendered with a name of
/// `type`.
///
/// [`UriDisplay<Query>`]: ../rocket/http/uri/trait.UriDisplay.html
/// [`Formatter::write_named_value()`]: ../rocket/http/uri/struct.Formatter.html#method.write_named_value
/// [`Formatter::write_value()`]: ../rocket/http/uri/struct.Formatter.html#method.write_value
#[proc_macro_derive(UriDisplayQuery, attributes(form))]
pub fn derive_uri_display_query(input: TokenStream) -> TokenStream {
    emit!(derive::uri_display::derive_uri_display_query(input))
}

/// Derive for the [`UriDisplay<Path>`] trait.
///
/// The [`UriDisplay<Path>`] derive can only be applied to tuple structs with
/// one field.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[derive(UriDisplayPath)]
/// struct Name(String);
///
/// #[derive(UriDisplayPath)]
/// struct Age(usize);
/// ```
///
/// The field's type is required to implement [`UriDisplay<Path>`].
///
/// The derive generates an implementation of the [`UriDisplay<Path>`] trait.
/// The implementation calls [`Formatter::write_value()`] for the field.
///
/// [`UriDisplay<Path>`]: ../rocket/http/uri/trait.UriDisplay.html
/// [`Formatter::write_value()`]: ../rocket/http/uri/struct.Formatter.html#method.write_value
#[proc_macro_derive(UriDisplayPath)]
pub fn derive_uri_display_path(input: TokenStream) -> TokenStream {
    emit!(derive::uri_display::derive_uri_display_path(input))
}

/// Generates a [`Vec`] of [`Route`]s from a set of route paths.
///
/// The `routes!` macro expands a list of route paths into a [`Vec`] of their
/// corresponding [`Route`] structures. For example, given the following routes:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// #[get("/")]
/// fn index() { /* .. */ }
///
/// mod person {
///     #[post("/hi/<person>")]
///     pub fn hello(person: String) { /* .. */ }
/// }
/// ```
///
/// The `routes!` macro can be used as:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # use rocket::http::Method;
/// #
/// # #[get("/")] fn index() { /* .. */ }
/// # mod person {
/// #   #[post("/hi/<person>")] pub fn hello(person: String) { /* .. */ }
/// # }
/// let my_routes = routes![index, person::hello];
/// assert_eq!(my_routes.len(), 2);
///
/// let index_route = &my_routes[0];
/// assert_eq!(index_route.method, Method::Get);
/// assert_eq!(index_route.name, Some("index"));
/// assert_eq!(index_route.uri.path(), "/");
///
/// let hello_route = &my_routes[1];
/// assert_eq!(hello_route.method, Method::Post);
/// assert_eq!(hello_route.name, Some("hello"));
/// assert_eq!(hello_route.uri.path(), "/hi/<person>");
/// ```
///
/// The grammar for `routes!` is defined as:
///
/// ```text
/// routes := PATH (',' PATH)*
///
/// PATH := a path, as defined by Rust
/// ```
///
/// [`Route`]: ../rocket/struct.Route.html
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    emit!(bang::routes_macro(input))
}

/// Generates a [`Vec`] of [`Catcher`]s from a set of catcher paths.
///
/// The `catchers!` macro expands a list of catcher paths into a [`Vec`] of
/// their corresponding [`Catcher`] structures. For example, given the following
/// catchers:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// #[catch(404)]
/// fn not_found() { /* .. */ }
///
/// mod inner {
///     #[catch(400)]
///     pub fn unauthorized() { /* .. */ }
/// }
/// ```
///
/// The `catchers!` macro can be used as:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # #[catch(404)] fn not_found() { /* .. */ }
/// # mod inner {
/// #     #[catch(400)] pub fn unauthorized() { /* .. */ }
/// # }
/// #
/// let my_catchers = catchers![not_found, inner::unauthorized];
/// assert_eq!(my_catchers.len(), 2);
///
/// let not_found = &my_catchers[0];
/// assert_eq!(not_found.code, 404);
///
/// let unauthorized = &my_catchers[1];
/// assert_eq!(unauthorized.code, 400);
/// ```
///
/// The grammar for `catchers!` is defined as:
///
/// ```text
/// catchers := PATH (',' PATH)*
///
/// PATH := a path, as defined by Rust
/// ```
///
/// [`Catcher`]: ../rocket/struct.Catcher.html
#[proc_macro]
pub fn catchers(input: TokenStream) -> TokenStream {
    emit!(bang::catchers_macro(input))
}

/// Type safe generation of route URIs.
///
/// The `uri!` macro creates a type-safe, URL safe URI given a route and values
/// for the route's URI parameters. The inputs to the macro are the path to a
/// route, a colon, and one argument for each dynamic parameter (parameters in
/// `<>`) in the route's path and query.
///
/// For example, for the following route:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// #[get("/person/<name>?<age>")]
/// fn person(name: String, age: Option<u8>) -> String {
/// # "".into() /*
///     ...
/// # */
/// }
/// ```
///
/// A URI can be created as follows:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # #[get("/person/<name>?<age>")]
/// # fn person(name: String, age: Option<u8>) { }
/// #
/// // with unnamed parameters, in route path declaration order
/// let mike = uri!(person: "Mike Smith", 28);
/// assert_eq!(mike.to_string(), "/person/Mike%20Smith?age=28");
///
/// // with named parameters, order irrelevant
/// let mike = uri!(person: name = "Mike", age = 28);
/// let mike = uri!(person: age = 28, name = "Mike");
/// assert_eq!(mike.to_string(), "/person/Mike?age=28");
///
/// // with a specific mount-point
/// let mike = uri!("/api", person: name = "Mike", age = 28);
/// assert_eq!(mike.to_string(), "/api/person/Mike?age=28");
///
/// // with unnamed values ignored
/// let mike = uri!(person: "Mike", _);
/// assert_eq!(mike.to_string(), "/person/Mike");
///
/// // with named values ignored
/// let mike = uri!(person: name = "Mike", age = _);
/// assert_eq!(mike.to_string(), "/person/Mike");
/// ```
///
/// ## Grammar
///
/// The grammar for the `uri!` macro is:
///
/// ```text
/// uri := (mount ',')? PATH (':' params)?
///
/// mount = STRING
/// params := unnamed | named
/// unnamed := expr (',' expr)*
/// named := IDENT = expr (',' named)?
/// expr := EXPR | '_'
///
/// EXPR := a valid Rust expression (examples: `foo()`, `12`, `"hey"`)
/// IDENT := a valid Rust identifier (examples: `name`, `age`)
/// STRING := an uncooked string literal, as defined by Rust (example: `"hi"`)
/// PATH := a path, as defined by Rust (examples: `route`, `my_mod::route`)
/// ```
///
/// ## Semantics
///
/// The `uri!` macro returns an [`Origin`] structure with the URI of the
/// supplied route interpolated with the given values. Note that `Origin`
/// implements `Into<Uri>` (and by extension, `TryInto<Uri>`), so it can be
/// converted into a [`Uri`] using `.into()` as needed.
///
/// A `uri!` invocation only typechecks if the type of every value in the
/// invocation matches the type declared for the parameter in the given route,
/// after conversion with [`FromUriParam`], or if a value is ignored using `_`
/// and the corresponding route type implements [`Ignorable`].
///
/// Each value passed into `uri!` is rendered in its appropriate place in the
/// URI using the [`UriDisplay`] implementation for the value's type. The
/// `UriDisplay` implementation ensures that the rendered value is URI-safe.
///
/// If a mount-point is provided, the mount-point is prepended to the route's
/// URI.
///
/// ### Conversion
///
/// The [`FromUriParam`] trait is used to typecheck and perform a conversion for
/// each value passed to `uri!`. If a `FromUriParam<P, S>` implementation exists
/// for a type `T` for part URI part `P`, then a value of type `S` can be used
/// in `uri!` macro for a route URI parameter declared with a type of `T` in
/// part `P`. For example, the following implementation, provided by Rocket,
/// allows an `&str` to be used in a `uri!` invocation for route URI parameters
/// declared as `String`:
///
/// ```rust,ignore
/// impl<P: UriPart, 'a> FromUriParam<P, &'a str> for String { .. }
/// ```
///
/// ### Ignorables
///
/// Query parameters can be ignored using `_` in place of an expression. The
/// corresponding type in the route URI must implement [`Ignorable`]. Ignored
/// parameters are not interpolated into the resulting `Origin`. Path parameters
/// are not ignorable.
///
/// [`Uri`]: ../rocket/http/uri/enum.Uri.html
/// [`Origin`]: ../rocket/http/uri/struct.Origin.html
/// [`FromUriParam`]: ../rocket/http/uri/trait.FromUriParam.html
/// [`UriDisplay`]: ../rocket/http/uri/trait.UriDisplay.html
/// [`Ignorable`]: ../rocket/http/uri/trait.Ignorable.html
#[proc_macro]
pub fn uri(input: TokenStream) -> TokenStream {
    emit!(bang::uri_macro(input))
}

#[doc(hidden)]
#[proc_macro]
pub fn rocket_internal_uri(input: TokenStream) -> TokenStream {
    emit!(bang::uri_internal_macro(input))
}
