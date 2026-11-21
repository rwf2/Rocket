+++
summary = "a migration guide from Rocket v0.5 to v0.6"
+++

# Upgrading

Rocket v0.6 brings many new features and improvements over Rocket v0.5,
including pluggable connection listeners, Unix domain sockets, structured
logging via `tracing`, and preview support for HTTP/3. Rocket v0.6 also
includes changes that make routing more precise and remove previously
deprecated APIs. While the Rust compiler can guide you through many of these
changes, others require special attention. The intent of this guide is to guide
you through these changes and more, migrating your Rocket application to 0.6
and reaping the benefits of new features and improvements.

This guide is _not_ intended to replace, but instead complement, a reading of
the [CHANGELOG]. The [CHANGELOG] should be considered required reading for all
developers wishing to migrate their applications to Rocket v0.6.

[CHANGELOG]: @git/master/CHANGELOG.md

! warning: Some changes are invisible to the compiler.

  Changes to [routing](#routing) and the [file server](#file-server) alter how
  requests are matched without causing any compiler errors. An application that
  compiles against v0.6 may nevertheless respond with a `404` where it
  previously succeeded. Read the [routing](#routing) section carefully and test
  your application's routes before deploying.

## Crate Versions

Rocket v0.6 is in development and has not been released. Its crates carry
`-dev` versions:

| crate                    | v0.5 version | v0.6 version |
|--------------------------|--------------|--------------|
| [`rocket`]               | `0.5.4`      | `0.6.0-dev`  |
| [`rocket_db_pools`]      | `0.3.0`      | `0.4.0-dev`  |
| [`rocket_dyn_templates`] | `0.3.0`      | `0.4.0-dev`  |
| [`rocket_sync_db_pools`] | `0.2.0`      | `0.3.0-dev`  |
| [`rocket_ws`]            | `0.2.0`      | `0.3.0-dev`  |

Development versions are not published, so they must be _git_ dependencies. For
example, to depend on `rocket` and `rocket_dyn_templates` at a given commit,
with `######` replaced by the commit hash, make the following changes to
`Cargo.toml`:

```diff
[dependencies]
- rocket = { version = "0.5.4", features = ["json"] }
- rocket_dyn_templates = { version = "0.3.0", features = ["tera"] }
+ rocket = { git = "https://github.com/rwf2/Rocket", rev = "######", features = ["json"] }
+ rocket_dyn_templates = { git = "https://github.com/rwf2/Rocket", rev = "######", features = ["tera"] }
```

Rocket v0.6 and all of its contrib libraries require Rust **1.95** or later.

[`rocket`]: @api/master/rocket/
[`rocket_db_pools`]: @api/master/rocket_db_pools/
[`rocket_dyn_templates`]: @api/master/rocket_dyn_templates/
[`rocket_sync_db_pools`]: @api/master/rocket_sync_db_pools/
[`rocket_ws`]: @api/master/rocket_ws/

### Default Features

Two new crate features, `trace` and `tokio-macros`, are enabled by default:

  * `trace` enables Rocket's default [tracing subscriber], which formats and
    prints log messages. Without it, Rocket emits no log output of its own.
  * `tokio-macros` enables the `macros` feature of the reexported
    [`rocket::tokio`], providing macros like `select!` and `join!`.

Applications that disable default features _must_ enable these features as
needed. To keep the previous defaults, enable them alongside `http2`:

```toml
[dependencies.rocket]
git = "https://github.com/rwf2/Rocket"
rev = "######"
default-features = false
features = ["http2", "trace", "tokio-macros"]
```

The reexported `tokio` also no longer enables `tokio`'s `io-std` feature.
Applications that use `rocket::tokio::io::stdin()` or similar _must_ depend on
`tokio` directly with the `io-std` feature enabled.

[tracing subscriber]: @api/master/rocket/trace/subscriber/index.html
[`rocket::tokio`]: @api/master/rocket/index.html#reexport.tokio

## Routing

Rocket v0.6 makes trailing slashes significant during routing. Where Rocket v0.5
disregarded a trailing slash in a request URI, Rocket v0.6 treats it as part of
the URI. This affects routing in the following ways:

  1. A request URI with a trailing slash no longer matches a route URI without
     one, and vice versa. A request to `/foo/` is not routed to `/foo`, and a
     request to `/foo/?a=1` is not routed to `/foo?<a>`.
  2. Route URIs may end with a trailing slash: `#[get("/foo/")]`. Such a route
     only matches requests with a trailing slash.
  3. A trailing slash is an empty final segment. As such, a request to `/user/`
     is routed to `/user/<name>` with an empty `name`. [`FromParam`] for `&str`,
     `String`, and numeric types fails on empty strings, so the request is
     forwarded with a `422` status where it previously failed to route with a
     `404`. Parameters of type `Option<T>` receive `None`.
  4. A route with a multi-segment parameter, like `/files/<path..>`, no longer
     matches a request to `/files`. Requests to `/files/` and below continue to
     match. A trailing slash is included in the matched path, so a request to
     `/files/a/` yields a `PathBuf` of `a/`.
  5. A route with URI `/` mounted at a base with a trailing slash, like `/foo/`,
     only matches `/foo/`. Mounted at `/foo`, it only matches `/foo`.
     Previously, both bases matched both requests.

For example, the following two routes now match different requests:

```rust
# use rocket::get;

#[get("/foo")]
fn foo() -> &'static str { "no slash" }

#[get("/foo/")]
fn foo_slash() -> &'static str { "slash" }
```

[`FromParam`]: @api/master/rocket/request/trait.FromParam.html

### Normalizing URIs

The [`AdHoc::uri_normalizer()`] fairing recovers most of the v0.5 behavior. When
attached, the fairing strips the trailing slash from any request URI that no
route with a trailing slash matches. It also mounts a copy of every route like
`/files/<path..>` at `/files` so that the prefix itself continues to match. The
fairing logs a warning whenever it rewrites a URI.

**The recommendation** is to use the fairing as a stop-gap measure while
updating routes and clients. Note that:

  * The fairing does not make a route mounted at a base with a trailing slash
    match requests without one. Mount such routes at the base without the slash
    instead.

  * The fairing is incompatible with the default [`FileServer`]. As described in
    the [file server](#file-server) section, `FileServer::new()` redirects
    requests for directories to a URI with a trailing slash, which the fairing
    then strips, resulting in a redirect loop. To use both, build a file server
    that doesn't redirect directories:

    ```rust
    # #[macro_use] extern crate rocket;
    use rocket::fairing::AdHoc;
    use rocket::fs::{FileServer, rewrite::{Prefix, DirIndex}};

    #[launch]
    fn rocket() -> _ {
        let files = FileServer::identity()
            .filter(|file, _| file.is_visible())
            .rewrite(Prefix::checked("static"))
            .rewrite(DirIndex::unconditional("index.html"));

        rocket::build()
            .mount("/", files)
            .attach(AdHoc::uri_normalizer())
    }
    ```

[`AdHoc::uri_normalizer()`]: @api/master/rocket/fairing/struct.AdHoc.html#method.uri_normalizer
[`FileServer`]: @api/master/rocket/fs/struct.FileServer.html

### Parameters

In addition to failing on empty strings, `FromParam` implementations report
errors with more precise types:

  * The error type for `&str` and `String` is now [`Empty`].
  * The error type for types parsed via `FromStr`, including integers, `bool`,
    and IP addresses, is now the type's `FromStr` error. Previously, it was the
    raw parameter string.

Routes that match on these errors _must_ be updated. To recover the raw string
when parsing fails, use [`Either`]:

```diff
+ use rocket::either::Either;

  #[get("/item/<id>")]
- fn item(id: Result<usize, &str>) { /* .. */ }
+ fn item(id: Either<usize, &str>) { /* .. */ }
```

[`Empty`]: @api/master/rocket/error/struct.Empty.html
[`Either`]: @api/master/rocket/index.html#reexport.either

### Route Attribute

The generic [`route`] attribute now takes the route URI first and the method as
an optional, named `method` parameter. Routes declared with `route` _must_ be
updated:

```diff
- #[route(GET, uri = "/")]
+ #[route("/", method = GET)]
  fn index() { /* .. */ }
```

A `route` without a `method` matches requests with _any_ method. Any method in
the IANA HTTP method registry can be named, either as an identifier like
`PROPFIND` or as a string like `"VERSION-CONTROL"`. The [methods] section of the
guide has examples.

[`route`]: @api/master/rocket/attr.route.html
[methods]: ../requests/#methods

### Methods

The [`Method`] enum now includes every method in the IANA HTTP method registry
and is marked `#[non_exhaustive]`. Accordingly:

  * `match` expressions on a `Method` _must_ include a wildcard arm.
  * `Method::from_str()` accepts only uppercase and lowercase method names:
    `"GET"` and `"get"` parse, but `"Get"` does not.
  * `Method::supports_payload()` is deprecated in favor of
    [`Method::allows_request_body()`], which returns `None` for methods whose
    specification doesn't define whether a request body is allowed.
  * [`Route::method`] is now an `Option<Method>`, where `None` matches any
    method. [`Route::new()`] accepts a `Method` or an `Option<Method>`.

[`Method`]: @api/master/rocket/http/enum.Method.html
[`Method::allows_request_body()`]: @api/master/rocket/http/enum.Method.html#method.allows_request_body
[`Route::method`]: @api/master/rocket/route/struct.Route.html#structfield.method
[`Route::new()`]: @api/master/rocket/route/struct.Route.html#method.new

### Mount Points

Mount points, or bases, are now treated like route URIs: they are never
percent-decoded and are instead compared against the percent-decoded request
path. Previously, a base had to be a valid origin URI.

  * [`Rocket::mount()`], [`Rocket::register()`], [`Route::rebase()`], and
    [`Catcher::rebase()`] take a `&str` base. Pass a `String` as `&base` and a
    URI as `uri.path().as_str()`.
  * A base may contain any UTF-8 characters. A route mounted at `/häcker`
    matches requests to `/h%C3%A4cker/..`.
  * A `%` in a catcher base is no longer a percent-encoding escape. A catcher
    registered at `/a%20b` now catches errors from requests to `/a%2520b/..`,
    which routes mounted at `/a%20b` have always matched, and no longer catches
    errors from requests to `/a%20b/..`.

[`Rocket::mount()`]: @api/master/rocket/struct.Rocket.html#method.mount
[`Rocket::register()`]: @api/master/rocket/struct.Rocket.html#method.register
[`Route::rebase()`]: @api/master/rocket/route/struct.Route.html#method.rebase
[`Catcher::rebase()`]: @api/master/rocket/catcher/struct.Catcher.html#method.rebase

## File Server

[`FileServer`] is now built from a pipeline of [rewrites] that map a request to
a file or redirect. `FileServer::from()`, the two-argument `FileServer::new()`,
and `fs::Options` have been removed. All users of `FileServer` _must_:

  * Replace `FileServer::from(path)` with `FileServer::new(path)`.
  * Replace uses of `Options` with the equivalent rewrites.

The new [`FileServer::new()`] serves visible files from `path`, serves
`index.html` for directories, and redirects requests for a directory without a
trailing slash to the same URI with a trailing slash. The redirect is new: with
`FileServer::from()`, the default in v0.5, `/static/dir` and `/static/dir/` both
served `dir/index.html`. Additionally, following the [routing](#routing)
changes, a file server mounted at `/static` no longer serves requests to
`/static` itself, and requests for files with a trailing slash, like
`/static/file.txt/`, no longer succeed.

Each `Options` value corresponds to one or more rewrites:

| v0.5 `Options`  | v0.6 equivalent                                                   |
|-----------------|-------------------------------------------------------------------|
| `None`          | `FileServer::identity()` with the filter and `Prefix` rewrite below |
| `Index`         | `.rewrite(DirIndex::unconditional("index.html"))`                 |
| `DotFiles`      | omit the `is_visible()` filter                                    |
| `NormalizeDirs` | `.rewrite(TrailingDirs)`                                          |
| `Missing`       | `Prefix::unchecked(path)` in place of `Prefix::checked(path)`      |

For example, a v0.5 file server that serves dotfiles but no index files becomes:

```diff
- use rocket::fs::{FileServer, Options};
+ use rocket::fs::{FileServer, rewrite::Prefix};

- FileServer::new("static", Options::DotFiles)
+ FileServer::identity().rewrite(Prefix::checked("static"))
```

[`FileServer::without_index()`] serves files exactly like `FileServer::new()`
without serving index files. Custom rewrites can be written as closures or by
implementing [`Rewriter`].

[rewrites]: @api/master/rocket/fs/rewrite/index.html
[`FileServer::new()`]: @api/master/rocket/fs/struct.FileServer.html#method.new
[`FileServer::without_index()`]: @api/master/rocket/fs/struct.FileServer.html#method.without_index
[`Rewriter`]: @api/master/rocket/fs/rewrite/trait.Rewriter.html

## Logging

Rocket v0.6 replaces the `log` crate with [`tracing`] for structured logging.
Applications that configure or inspect logging _must_:

  * Replace `log_level` configuration values according to the table below.
  * Replace references to `rocket::config::LogLevel` with [`Level`]. The
    `Config::log_level` field is now an `Option<Level>`, where `None` disables
    logging.
  * Use [`CliColors`] where `Config::cli_colors` was used as a `bool`.

The `log_level` parameter now accepts `tracing` level names or a number from `0`
to `5`. The v0.5 names `critical` and `normal` result in a configuration error
at launch:

| v0.5 `log_level` | v0.6 `log_level` |
|------------------|------------------|
| `off`            | `off` or `0`     |
| `critical`       | `warn` or `2`    |
| `normal`         | `info` or `3`    |
| `debug`          | `trace` or `5`   |

The `error` (`1`) and `debug` (`4`) levels are new. The default level in release
mode is now `error`, which hides warnings. Set `log_level` to `warn` to see
warnings in release mode as before.

The `cli_colors` parameter continues to accept `true` and `false` and
additionally accepts `"always"`, `"auto"`, and `"never"`. The new `log_format`
parameter selects between `"pretty"` output, the default in debug mode, and
`"compact"` output, the default in release mode.

[`tracing`]: https://docs.rs/tracing/0.1/tracing/
[`Level`]: @api/master/rocket/config/struct.Level.html
[`CliColors`]: @api/master/rocket/config/enum.CliColors.html

### Logging Macros

With `#[macro_use] extern crate rocket`, the `error!`, `warn!`, `info!`,
`debug!`, and `trace!` macros now emit `tracing` events, so existing uses
continue to compile. The macros accept `tracing`'s structured fields:

```rust
# #[macro_use] extern crate rocket;

#[get("/hello/<name>")]
fn hello(name: &str) -> String {
    info!(name, "saying hello");
    format!("Hello, {name}!")
}
```

The `tracing` crate itself is reexported as [`rocket::tracing`]. The indented
variants of the logging macros, like `info_!`, and the `rocket::log` module
have been removed. To emit `log` records instead, depend on `log` directly:
with the `trace` feature enabled, Rocket's subscriber forwards `log` records.

### Custom Subscribers

Rocket installs its subscriber when an instance of `Rocket` is first created,
for instance, via `rocket::build()`, unless a global subscriber has already been
installed. To use your own subscriber, install it before creating an instance
of `Rocket`. Rocket's subscriber can also be used as a layer in your own
subscriber via the [`trace::subscriber`] module.

[`rocket::tracing`]: @api/master/rocket/index.html#reexport.tracing
[`trace::subscriber`]: @api/master/rocket/trace/subscriber/index.html

## Configuration

Rocket v0.6 moves the configuration of network interfaces out of [`Config`] and
into [connection listeners]. The `address`, `port`, and `tls` parameters are
still read from the configuration provider, now by the listener. All users
_must_:

  * Remove uses of the `Config::address`, `Config::port`, and `Config::tls`
    fields and the `Config::ADDRESS`, `Config::PORT`, and `Config::TLS`
    constants. Set these parameters via the configuration provider and read the
    bound endpoints via [`Rocket::endpoints()`].
  * Replace `Config::tls_enabled()` and `Config::mtls_enabled()` with a check
    of the bound endpoints, like `rocket.endpoints().any(|e| e.is_tls())`.
  * Replace calls to `Rocket::configure()` with [`Rocket::reconfigure()`].
  * Replace `rocket::config::Shutdown` with [`ShutdownConfig`],
    `rocket::config::MutualTls` with [`MtlsConfig`], and
    `rocket::config::CipherSuite` with [`tls::CipherSuite`].

A configuration that previously set the port via `Config` becomes:

```diff
  use rocket::config::Config;

  #[launch]
  fn rocket() -> _ {
-     let config = Config {
-         port: 1111,
-         ..Config::debug_default()
-     };
+     let figment = Config::figment().merge(("port", 1111));

-     rocket::custom(config)
+     rocket::custom(figment)
  }
```

To read the address that Rocket is serving on, use [`Rocket::endpoints()`] once
the instance is in orbit, for instance, in a liftoff fairing:

```rust
# #[macro_use] extern crate rocket;
use rocket::fairing::AdHoc;

#[launch]
fn rocket() -> _ {
    rocket::build().attach(AdHoc::on_liftoff("Endpoints", |rocket| Box::pin(async move {
        for endpoint in rocket.endpoints() {
            info!(%endpoint, "listening");
        }
    })))
}
```

[`Config`]: @api/master/rocket/struct.Config.html
[connection listeners]: @api/master/rocket/listener/index.html
[`Rocket::endpoints()`]: @api/master/rocket/struct.Rocket.html#method.endpoints
[`Rocket::reconfigure()`]: @api/master/rocket/struct.Rocket.html#method.reconfigure
[`ShutdownConfig`]: @api/master/rocket/shutdown/struct.ShutdownConfig.html
[`MtlsConfig`]: @api/master/rocket/mtls/struct.MtlsConfig.html
[`tls::CipherSuite`]: @api/master/rocket/tls/enum.CipherSuite.html

### Addresses

The `address` parameter is now an [`Endpoint`]. In addition to an IP address,
it accepts a socket address, which includes a port, and a Unix domain socket
path prefixed with `unix:`. A `port`, if set, replaces the port in a TCP
`address`. For example, to serve on a Unix domain socket:

```toml
[default]
address = "unix:/run/app/rocket.sock"
```

[`Endpoint`]: @api/master/rocket/listener/enum.Endpoint.html

### TLS

TLS configuration in `Rocket.toml` is unchanged. TLS types are now in the
[`tls`] module, mutual TLS types in the [`mtls`] module, and [`TlsConfig`]
remains available as `rocket::config::TlsConfig`. Beyond the renames above:

  * Key files _must_ contain exactly one private key and no other items. A key
    file containing, for instance, both a key and a certificate is rejected at
    launch. Previously, Rocket used the first supported key and ignored any
    other contents.
  * A [`CryptoProvider`] installed as the process default is now used for TLS
    and replaces the configured `ciphers`. Previously, Rocket always used `ring`
    with the configured `ciphers`. See [crypto providers] for details.
  * The [`mtls::x509`], [`mtls::oid`], and [`mtls::bigint`] modules now
    reexport types from `x509-parser` `0.18`, up from `0.13`. Applications
    that use these types or depend on `x509-parser`, `der-parser`,
    `oid-registry`, or `asn1-rs` directly may need to update.

[`tls`]: @api/master/rocket/tls/index.html
[`mtls`]: @api/master/rocket/mtls/index.html
[`TlsConfig`]: @api/master/rocket/tls/struct.TlsConfig.html
[`CryptoProvider`]: @rustls/crypto/struct.CryptoProvider.html
[crypto providers]: ../configuration/#crypto-providers
[`mtls::x509`]: @api/master/rocket/mtls/x509/index.html
[`mtls::oid`]: @api/master/rocket/mtls/oid/index.html
[`mtls::bigint`]: @api/master/rocket/mtls/bigint/index.html

## Errors and Launch

A [`rocket::Error`] no longer panics when dropped without being inspected. The
`#[launch]` attribute now generates a `main` function that reports launch
errors with [`Error::report()`] and returns an [`ExitCode`]. As a result, a
failed launch logs the error and exits with a failure status. Previously, it
panicked.

Applications that launch Rocket manually _should_ do the same:

```rust,no_run
use std::process::ExitCode;
use rocket::Error;

#[rocket::main]
async fn main() -> ExitCode {
    Error::report(rocket::build().launch().await)
}
```

`Error::pretty_print()` has been removed. To log an error, use
[`Trace::trace_error()`]. Additionally, the variants of [`ErrorKind`] have
changed:

  * `Bind` carries the endpoint, if known, and a boxed error.
  * `Collisions` is a struct variant with `routes` and `catchers` fields.
  * `Shutdown` carries only the instance of `Rocket`.
  * `Liftoff` is new and indicates that liftoff failed, for instance, because a
    liftoff fairing panicked.

[`rocket::Error`]: @api/master/rocket/error/struct.Error.html
[`Error::report()`]: @api/master/rocket/error/struct.Error.html#method.report
[`ExitCode`]: @std/process/struct.ExitCode.html
[`Trace::trace_error()`]: @api/master/rocket/trace/trait.Trace.html#method.trace_error
[`ErrorKind`]: @api/master/rocket/error/enum.ErrorKind.html

## HTTP and Connections

The `rocket::http::hyper` module, which reexported `hyper` and `http` types, has
been removed. Applications that used it, for example, for header name constants,
_must_ depend on `hyper` or `http` directly or use Rocket's own types.

Several APIs that dealt with connections now work with an [`Endpoint`] in place
of a `SocketAddr`, so that they work with any listener:

  * [`Request::remote()`] returns an `Option<&Endpoint>`, and
    `Request::set_remote()` takes an `Endpoint`. Use [`Endpoint::tcp()`] to get
    a `SocketAddr`. The `SocketAddr` request guard is unchanged, and an
    `&Endpoint` request guard is new.
  * `LocalRequest::remote()` accepts anything that converts into an `Endpoint`,
    including a `SocketAddr`.

```diff
- let ip = request.remote().map(|addr| addr.ip());
+ let ip = request.remote().and_then(|endpoint| endpoint.ip());
```

Custom connection upgrades _must_ update their [`IoHandler`] implementations:
`IoHandler::io()` now takes `self: Box<Self>`, and [`Response::upgrade()`]
returns an `Option<&mut dyn IoHandler>`.

```diff
  #[rocket::async_trait]
  impl IoHandler for MyHandler {
-     async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
+     async fn io(self: Box<Self>, io: IoStream) -> io::Result<()> {
          /* .. */
      }
  }
```

[`Request::remote()`]: @api/master/rocket/request/struct.Request.html#method.remote
[`Endpoint::tcp()`]: @api/master/rocket/listener/enum.Endpoint.html#method.tcp
[`IoHandler`]: @api/master/rocket/data/trait.IoHandler.html
[`Response::upgrade()`]: @api/master/rocket/response/struct.Response.html#method.upgrade

### Other API Changes

The following changes are less likely to affect applications:

  * `Client::new()`, deprecated since v0.5.0, has been removed. Use
    [`Client::tracked()`] or `Client::untracked()`.
  * The fields of [`RouteUri`] are private. Use `RouteUri::base()`,
    `RouteUri::inner()`, and `RouteUri::unmounted()`. Similarly, use
    [`Catcher::base()`] in place of the `Catcher::base` field.
  * `Shutdown::notify()` takes `&self`.
  * `Segments::len()` has been renamed to [`Segments::num()`].
  * `rocket::Either` is now [`rocket::either::Either`].
  * The `IntoCollection` trait has been removed. APIs that accepted one,
    like `Accept::new()`, accept an `IntoIterator`.

[`Client::tracked()`]: @api/master/rocket/local/blocking/struct.Client.html#method.tracked
[`RouteUri`]: @api/master/rocket/route/struct.RouteUri.html
[`Catcher::base()`]: @api/master/rocket/catcher/struct.Catcher.html#method.base
[`Segments::num()`]: @api/master/rocket/http/uri/struct.Segments.html#method.num
[`rocket::either::Either`]: @api/master/rocket/index.html#reexport.either

## Typed URIs

URIs generated by [`uri!`] are now percent-encoded and normalized in the same
way as route URIs:

  * Static segments of a route URI are percent-encoded. For a route URI of
    `/a%20b`, `uri!` now generates `/a%2520b`, which is the request URI the
    route matches.
  * Literal prefixes starting with `/` use mount point syntax and are encoded
    the same way: a prefix of `/a%20b` generates `/a%2520b`. To use an already
    encoded path, pass a URI expression: `uri!(uri!("/a%20b"), handler)`.
    Absolute URI prefixes are unchanged.
  * Static query segments are percent-encoded. A static query of `c=d+e` now
    generates `c=d%2Be`.
  * URI literals that are not normalized are rejected at compile-time. For
    example, `uri!("/a//b")` _must_ be written as `uri!("/a/b")`.
  * The generated URI is normalized after route arguments are substituted and
    any prefix or suffix is added. For a route URI of `/<a>/<b>`, the values
    `""` and `"b"` now generate `/b` where they previously generated `//b`.

[`uri!`]: @api/master/rocket/macro.uri.html

## Behavior Changes

The following changes affect the behavior of applications without requiring
any code changes:

  * **Requests are cancelled when the shutdown grace period ends.**

    During graceful shutdown, request handlers that have not completed by the
    end of the [grace period] are cancelled. Previously, they continued to run
    until they completed or the async runtime shut down.

  * **Malformed requests are processed like any other.**

    Requests that fail to parse, which result in a `400` error, now pass through
    request fairings, the `400` catcher, and response fairings. Previously,
    only the catcher ran.

  * **MessagePack responses use named encoding.**

    The [`MsgPack`] responder now serializes structures as maps with field
    names. Previously, it serialized them as arrays. To use the previous
    encoding, respond with [`Compact`]:

    ```diff
    - use rocket::serde::msgpack::MsgPack;
    + use rocket::serde::msgpack::Compact;

      #[get("/user")]
    - fn user() -> MsgPack<User> {
    + fn user() -> Compact<User> {
    ```

    `MsgPack<T>` continues to deserialize both encodings.

[grace period]: @api/master/rocket/shutdown/struct.ShutdownConfig.html#structfield.grace
[`MsgPack`]: @api/master/rocket/serde/msgpack/struct.MsgPack.html
[`Compact`]: @api/master/rocket/serde/msgpack/type.Compact.html

## Contrib

The contrib libraries are largely unchanged. Note the following:

  * **`rocket_db_pools`: SQLx pools connect lazily.**

    Previously, SQLx pools connected to the database during ignition, so an
    unreachable or misconfigured database aborted launch. Pools now connect
    when a connection is first requested, and connection errors are reported
    then. To abort launch if the database cannot be reached, acquire a
    connection in an ignite fairing:

    ```rust
    # #[macro_use] extern crate rocket;
    use rocket::fairing::AdHoc;
    use rocket_db_pools::{sqlx, Database};

    #[derive(Database)]
    #[database("sqlite_db")]
    struct Db(sqlx::SqlitePool);

    #[launch]
    fn rocket() -> _ {
        rocket::build()
            .attach(Db::init())
            .attach(AdHoc::try_on_ignite("Connect to Database", |rocket| async {
                match Db::fetch(&rocket) {
                    Some(db) if db.acquire().await.is_ok() => Ok(rocket),
                    _ => Err(rocket),
                }
            }))
    }
    ```

  * **`rocket_db_pools`: The `Pool` trait no longer requires `Sync`.**

    Code that is generic over [`Pool`] and requires `Sync` _must_ add the bound
    explicitly.

  * **`rocket_ws`: Streams complete the closing handshake.**

    When a stream returned from a WebSocket handler ends, `rocket_ws` now sends
    a `Close` message and waits up to 5 seconds for the client to reply before
    closing the connection. Previously, it closed the connection without sending
    a `Close` message.

`rocket_db_pools` also adds support for Diesel's SQLite backend via the new
`diesel_sqlite` feature and [`diesel::SqlitePool`].

[`Pool`]: @api/master/rocket_db_pools/trait.Pool.html
[`diesel::SqlitePool`]: @api/master/rocket_db_pools/diesel/type.SqlitePool.html

## Notable New Features

Rocket v0.6 brings many new features that enable new functionality and give
applications more control over how they are served. We encourage all users to
take advantage of these new features.

### Connection Listeners

Rocket v0.6 introduces pluggable [connection listeners]. A listener accepts
connections from any source; Rocket handles the HTTP. Rocket provides listeners
for TCP, Unix domain sockets, and TLS over either, and selects among them
according to configuration, as described in [addresses](#addresses).

A custom listener implements [`Listener`] and, to be configurable, [`Bind`].
Rocket can launch on a listener that implements `Bind` with
[`Rocket::launch_with()`] or on an existing listener with
[`Rocket::launch_on()`].

[`Listener`]: @api/master/rocket/listener/trait.Listener.html
[`Bind`]: @api/master/rocket/listener/trait.Bind.html
[`Rocket::launch_with()`]: @api/master/rocket/struct.Rocket.html#method.launch_with
[`Rocket::launch_on()`]: @api/master/rocket/struct.Rocket.html#method.launch_on

### HTTP/3

Rocket v0.6 includes experimental, preview support for [HTTP/3] over QUIC. To
enable it, enable the `http3-preview` feature and configure TLS:

```toml
[dependencies]
rocket = { git = "https://github.com/rwf2/Rocket", rev = "######", features = ["http3-preview"] }
```

When TLS is configured, Rocket serves HTTP/3 alongside HTTP/1 and HTTP/2 on the
same port and advertises it via the `Alt-Svc` header. HTTP/3 requires the
`TLS_AES_128_GCM_SHA256` cipher suite to be enabled.

[HTTP/3]: @api/master/rocket/listener/quic/index.html

### Tracing

Rocket's logging is now based on [`tracing`]. Rocket emits structured events
and spans, including a span for each request, and its default subscriber
formats them in `pretty` or `compact` form. Applications can emit events with
the reexported `tracing` macros and use any `tracing` subscriber. The [`trace`]
module contains Rocket's tracing facilities.

[`trace`]: @api/master/rocket/trace/index.html

### XML

The new `xml` feature adds [XML] support in the same style as JSON and
MessagePack: `Xml<T>` is a data guard, a form guard, and a responder, and the
local client provides `LocalRequest::xml()` and `LocalResponse::into_xml()`.

[XML]: @api/master/rocket/serde/xml/index.html

### Lint Suppression

Rocket's code generation emits warnings for questionable code, such as an
unknown media type in a `format` or a `GET` route that accepts a body. The new
[`suppress`] attribute silences these warnings:

```rust
# #[macro_use] extern crate rocket;

#[suppress(dubious_payload)]
#[get("/", data = "<body>")]
fn index(body: &str) { /* .. */ }
```

[`suppress`]: @api/master/rocket/attr.suppress.html

### Enum Parameters

[`FromParam`] can now be derived for enums whose variants have no fields. The
derived implementation matches each variant by its name:

```rust
# #[macro_use] extern crate rocket;

#[derive(FromParam)]
enum Theme {
    Light,
    Dark,
}

#[get("/theme/<theme>")]
fn theme(theme: Theme) { /* .. */ }
```

### And More

Rocket v0.6 includes several other additions:

  * [Fairing retrieval]: `Rocket::fairing()` and friends return references to
    attached fairings by type.
  * [Proxied TLS]: the `proxy_proto_header` parameter identifies requests that
    a proxy received over TLS, for which cookies are `Secure` by default.
  * [Dynamic TLS]: a [`Resolver`] selects TLS configuration per connection.
  * [Wake interval]: the `wake_interval` parameter configures Rocket's runtime
    waker, which keeps I/O and timers progressing when a task fails to yield.
  * [Data transforms]: `Data::chain_transform()` and friends transform request
    body data as it is read.
  * Request information: [`Request::sni()`] and [`Request::version()`] return
    the TLS server name and HTTP version of the request.
  * Route introspection: `Route::matches()` and `Route::collides_with()`, and
    their `Catcher` analogs, test requests and collisions.

[Fairing retrieval]: @api/master/rocket/struct.Rocket.html#method.fairing
[Proxied TLS]: ../configuration/#proxied-tls
[Dynamic TLS]: @api/master/rocket/tls/trait.Resolver.html
[`Resolver`]: @api/master/rocket/tls/trait.Resolver.html
[Wake interval]: ../configuration/#wake-interval
[Data transforms]: @api/master/rocket/data/struct.Data.html#method.chain_transform
[`Request::sni()`]: @api/master/rocket/request/struct.Request.html#method.sni
[`Request::version()`]: @api/master/rocket/request/struct.Request.html#method.version

## Getting Help

If you run into any issues upgrading, we encourage you to ask questions via
[GitHub discussions] or via chat at [`#rocket:mozilla.org`] on Matrix. The
[FAQ](../faq/) also provides answers to commonly asked questions.

[GitHub discussions]: @github/discussions
[`#rocket:mozilla.org`]: @chat
