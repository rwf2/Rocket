# Version 0.6.0-rc.1 (Unreleased)

## Major Features and Improvements

This release introduces the following major features and improvements:

  * Pluggable [connection listeners] and support for serving on [Unix domain sockets].
  * Structured logging and tracing based on [`tracing`] with `pretty` and `compact` formats.
  * Experimental, preview support for [HTTP/3] over QUIC via the `http3-preview` feature.
  * More precise routing: trailing slashes in request and route URIs are significant.
  * A [`FileServer`] built from a pipeline of composable, customizable [rewrites].
  * [Dynamic TLS] configuration selected per connection by a [`Resolver`].
  * Support for any installed [`CryptoProvider`] for TLS.
  * [Routes for any method]: every method in the IANA HTTP method registry and method-less routes.
  * [XML] data and form guards and responders via the new `xml` feature.
  * Chainable [data transformers] that transform request body data as it is read.
  * [`#[suppress]`](https://api.rocket.rs/master/rocket/attr.suppress.html) for silencing
    Rocket's lints.
  * [`#[derive(FromParam)]`](https://api.rocket.rs/master/rocket/derive.FromParam.html) for enums
    whose variants have no fields.
  * Mount points with any UTF-8 characters and consistent percent-encoding in [`uri!`].
  * A configurable [proxy protocol header] to identify requests a proxy received over TLS.
  * A runtime [waker] that keeps I/O and timers progressing when a task blocks a worker.
  * Exit codes for failed launches via [`Error::report()`].
  * [Fairing retrieval] by type from an instance of `Rocket`.
  * Public [route matching] and collision checking for routes and catchers.

## Breaking Changes

This release includes many breaking changes. The most significant changes are listed below. The
[upgrading guide] describes how to handle them.

### Silent Changes

These changes are invisible to the compiler and will _not_ yield errors or warnings at compile-time.
We **strongly** advise all application authors to review this list carefully.

  * Trailing slashes are significant during routing. A request to `/foo/` is not routed to `/foo`,
    and a request to `/foo/?a=1` is not routed to `/foo?<a>`. Attach [`AdHoc::uri_normalizer()`]
    to strip the trailing slash from requests that would otherwise fail to route.
  * A trailing slash is an empty final segment. A request to `/user/` is routed to `/user/<name>`
    with an empty `name`. `FromParam` for `&str`, `String`, and numeric types fails on empty
    segments, so the request is forwarded with a `422` where it previously failed to route with a
    `404`. Parameters of type `Option<T>` receive `None`.
  * A route with a multi-segment parameter, like `/files/<path..>`, no longer matches a request to
    `/files`. A trailing slash is included in the matched path: `/files/a/` yields `a/`.
  * A route with URI `/` mounted at a base with a trailing slash, like `/foo/`, only matches
    `/foo/`. Mounted at `/foo`, it only matches `/foo`.
  * [`FileServer::new()`], which replaces `FileServer::from()`, redirects requests for a directory
    without a trailing slash to the same URI with a trailing slash. A file server mounted at
    `/static` no longer serves requests to `/static`.
  * [`uri!`] percent-encodes static path and query segments and literal prefixes that begin with
    `/`. For a route URI of `/a%20b`, `uri!` generates `/a%2520b`, the request URI the route
    matches. A static query of `c=d+e` generates `c=d%2Be`.
  * [`uri!`] normalizes the generated URI after arguments are substituted. For a route URI of
    `/<a>/<b>`, the values `""` and `"b"` generate `/b` where they previously generated `//b`.
  * A `%` in a catcher base is no longer a percent-encoding escape. A catcher registered at `/a%20b`
    catches errors from requests to `/a%2520b/..`, which routes mounted at `/a%20b` have always
    matched.
  * The default `log_level` in release mode is now `error`, which hides warnings. Set `log_level`
    to `warn` to see warnings as before.
  * The `log_level` values `critical` and `normal` result in a configuration error at launch.
  * `Method::from_str()` no longer parses mixed-case method names: `"Get"` fails to parse.
  * Request handlers that have not completed by the end of the shutdown grace period are cancelled.
  * Requests that fail to parse pass through request fairings, the `400` catcher, and response
    fairings, both in the server and in local clients. Previously, only the catcher ran.
  * The [`MsgPack`] responder serializes structures as maps with field names. Respond with
    [`Compact`] for the previous array encoding. `MsgPack<T>` deserializes both encodings.
  * TLS key files must contain exactly one private key and no other items. Previously, the first
    supported key was used, and any other contents were ignored.
  * A [`CryptoProvider`] installed as the process default is used for TLS and replaces the
    configured `ciphers`. Previously, Rocket always used `ring` with the configured `ciphers`.
  * The `main` function generated by `#[launch]` logs launch errors and exits with a failure
    status. Previously, it panicked.
  * [`Error`] no longer panics when dropped without being inspected.
  * `rocket_db_pools` SQLx pools connect when a connection is first requested. Previously, a
    database that couldn't be reached during ignition aborted launch.
  * When a `rocket_ws` [`MessageStream`] ends, a `Close` message is sent, if one wasn't yielded,
    and the client's reply is awaited for up to 5 seconds before the connection is closed.

### General

  * The minimum supported Rust version for all crates is now `1.95`.
  * The new `trace` and `tokio-macros` features are enabled by default. Applications that disable
    default features must enable them as needed.
  * The reexported `tokio` no longer enables the `io-std` feature and only enables the `macros`
    feature with the `tokio-macros` feature.
  * The configuration of network interfaces moved to [connection listeners]:
    * Removed `Config::address`, `Config::port`, `Config::tls`, and their constants.
    * Removed `Config::tls_enabled()` and `Config::mtls_enabled()`.
    * The `address`, `port`, and `tls` parameters are read by the listener.
    * Added [`Rocket::endpoints()`] to retrieve the endpoints Rocket is listening on.
  * Renamed `Rocket::configure()` to [`Rocket::reconfigure()`].
  * Renamed `config::Shutdown` to [`ShutdownConfig`] and moved it to the new [`rocket::shutdown`]
    module. It is reexported from `config`.
  * Renamed `config::MutualTls` to [`MtlsConfig`] and moved it to `rocket::mtls`.
  * Moved `config::CipherSuite` to [`tls::CipherSuite`].
  * Replaced `log` with [`tracing`]:
    * Replaced `config::LogLevel` with [`Level`]. `Config::log_level` is an `Option<Level>`, where
      `None` disables logging.
    * The `log_level` parameter accepts `off`, `error`, `warn`, `info`, `debug`, `trace`, or `0`
      through `5`.
    * `Config::cli_colors` is a [`CliColors`].
    * The `error!`, `warn!`, `info!`, `debug!`, and `trace!` macros emit `tracing` events.
    * Removed the indented logging macros, like `info_!`, and the `rocket::log` module.
    * Removed `Error::pretty_print()`. Use [`Trace::trace_error()`] instead.
    * `Route`, `Catcher`, `Request`, `Outcome`, `Limits`, and `config::Shutdown` no longer
      implement `Display`. `Route`, `Catcher`, and `Request` implement [`Trace`] instead.
  * Changed the variants of [`ErrorKind`]:
    * `Bind` carries the endpoint, if known, and a boxed error.
    * `Collisions` is a struct variant with `routes` and `catchers` fields.
    * `Shutdown` carries only the instance of `Rocket`.
    * Added `Liftoff`, which indicates that liftoff failed.
  * Removed `Client::new()`, deprecated since `0.5.0`.
  * Moved `rocket::Either` to [`rocket::either::Either`].
  * Removed the `rocket::http::hyper` module.
  * `Shutdown::notify()` takes `&self`.

### Routing and URIs

  * The [`route`] attribute takes the route URI first and an optional `method`:
    `#[route(GET, uri = "/")]` is now `#[route("/", method = GET)]`.
  * [`Method`] has a variant for every method in the IANA HTTP method registry and is marked
    `#[non_exhaustive]`.
  * Deprecated `Method::supports_payload()` in favor of [`Method::allows_request_body()`].
  * `Route::method` is an `Option<Method>`, where `None` matches any method. `Route::new()` and
    `Route::ranked()` accept a `Method` or an `Option<Method>`.
  * [`Rocket::mount()`] and [`Rocket::register()`] take a `&str` base. Bases are never
    percent-decoded.
  * The fields of [`RouteUri`] are private. Use `RouteUri::base()`, `RouteUri::inner()`, and
    `RouteUri::unmounted()`. `RouteUri` dereferences to an `Origin`, whose methods replace
    `RouteUri::as_str()`, `RouteUri::path()`, and `RouteUri::query()`.
  * Replaced the `Catcher::base` field with [`Catcher::base()`].
  * `RouteUri::base()` and `Catcher::base()` return a `Path`.
  * Renamed `Segments::len()` to `Segments::num()`.
  * [`uri!`] rejects URI literals that are not normalized: `uri!("/a//b")` must be written as
    `uri!("/a/b")`.
  * `FromParam` for `&str` and `String` fails with an [`Empty`] error on empty segments.
  * The `FromParam` error for types parsed via `FromStr` is the type's `FromStr` error.
    Previously, it was the raw segment.
  * Removed the `IntoCollection` trait. APIs that accepted one, like `Accept::new()`, accept an
    `IntoIterator`.

### Data and Forms

  * `DataStream` is now a `#[non_exhaustive]` enum.

### Request Guards

  * `Request::remote()` returns an `Option<&Endpoint>`, and `Request::set_remote()` takes an
    [`Endpoint`]. The `SocketAddr` request guard is unchanged.
  * `LocalRequest::remote()` accepts any type that converts into an `Endpoint`.

### Responders

  * Removed `FileServer::from()`, the two-argument `FileServer::new()`, and `fs::Options` in favor
    of [rewrites].
  * `IoHandler::io()` takes `self: Box<Self>`.
  * `Response::upgrade()` returns an `Option<&mut dyn IoHandler>`.

### Contrib Libraries

  * The `rocket_db_pools` [`Pool`] trait no longer requires `Sync`. ([#2969])

## General Improvements

In addition to new features and major improvements, Rocket saw the following improvements:

### General

  * Added [`AdHoc::uri_normalizer()`], which strips trailing slashes from requests that would
    otherwise fail to route.
  * Added support for trailing slashes in route URIs: `#[get("/foo/")]`.
  * Added support for mount points with any UTF-8 characters. A route mounted at `/häcker` matches
    requests to `/h%C3%A4cker/..`.
  * Added `#[suppress]` for silencing Rocket's lints. ([#1188])
  * Added `#[derive(FromParam)]` for enums and the [`InvalidOption`] error it returns.
  * Added [`Rocket::fairing()`], `Rocket::fairing_mut()`, `Rocket::fairings()`, and
    `Rocket::fairings_mut()`.
  * Added [`Rocket::launch_with()`], [`Rocket::launch_on()`], and [`Rocket::try_launch_on()`].
  * Added [`Error::report()`].
  * Added `Route::matches()`, `Route::collides_with()`, `Catcher::matches()`, and
    `Catcher::collides_with()`. ([#1561])
  * Added [`Route::rebase()`] and [`Catcher::rebase()`]. ([#2533])
  * Added support for `Either` in `uri!`.
  * Added the [`wake_interval`] configuration parameter. ([#2239])
  * Added the [`proxy_proto_header`] configuration parameter, [`Request::proxy_proto()`], and
    [`Request::context_is_likely_secure()`].
  * Added the `log_format` configuration parameter.
  * Added support for `"always"`, `"auto"`, and `"never"` values for `cli_colors`. ([#2649])
  * Added [`Data::chain_transform()`] and friends for transforming request body data. ([#775])
  * Added the [`rocket::trace`] module. ([#21])
    * Added the [`Trace`] and `TraceAll` traits.
    * Added the [`RocketFmt`] and [`RocketDynFmt`] subscribers. ([#2840])
    * Added [`RequestId`], which tags each request's span with a unique ID.
    * Logs include the source location of routes and catchers.
    * Rocket's subscriber forwards `log` records.
  * Added the `xml` feature and the [`rocket::serde::xml`] module.
  * Added `Limits::XML`, set to 1 MiB by default.
  * `MsgPack` is generic over its encoding. [`Compact`] responds with compact MessagePack.
    ([#2107])
  * Fixed route attributes in `macro_rules!` macros, which failed to compile with "cannot find
    value `__req` in this scope". ([#2919])

### HTTP

  * Added preview support for HTTP/3 via the `http3-preview` feature. ([#2723])
  * Added `Method` variants for every method in the IANA HTTP method registry. ([#232])
  * Added support for routes that match any method: `#[route("/")]`. ([#2731])
  * Added `Method::is_safe()`, `Method::is_idempotent()`, and `Method::allows_request_body()`.
  * Added [`HttpVersion`] and [`Request::version()`].
  * Added [`ProxyProto`].
  * Trailing slashes and empty queries are allowed in normalized URIs: `/foo/` and `/foo?` are
    normalized. ([#2512])
  * Added `Origin::has_trailing_slash()`, `Origin::is_normalized_nontrailing()`, and
    `Origin::into_normalized_nontrailing()`.
  * Added `Uri::is_normalized()`, `Uri::normalize()`, and `Uri::into_normalized()`.
  * Added `Authority::set_port()`, `RawStr::trim()`, and `Accept::add()`.
  * `RawStr::split()` and `Path::raw_segments()` return a `DoubleEndedIterator`.

### Connections and TLS

  * Added pluggable [connection listeners] via the [`Listener`] and [`Bind`] traits. ([#2671])
  * Added support for Unix domain sockets: an `address` of `unix:path/to/socket`. ([#1070])
  * Added the `reuse` configuration parameter for Unix domain sockets.
  * Added the [`Endpoint`] type.
  * Added the [`rocket::tls`] module, including [`TlsListener`].
  * Added [`Resolver`] for selecting TLS configuration per connection. ([#2730], [#2363], [#2748],
    [#2683], [#2577])
  * Added support for using an installed [`CryptoProvider`].
  * Added `TlsConfig::validate()`, `TlsConfig::server_config()`, `TlsConfig::certs_reader()`,
    `TlsConfig::key_reader()`, and `MtlsConfig::ca_certs_reader()`.
  * Added [`Request::sni()`]. ([#2942], [#3015])

### Request

  * Added an `&Endpoint` request guard.
  * Added [`LocalRequest::xml()`] and [`LocalResponse::into_xml()`].
  * `LocalRequest::json()`, `LocalRequest::msgpack()`, and `LocalRequest::xml()` replace an
    existing `Content-Type` header. Previously, they added a second one, and the first remained in
    effect.

### Response

  * Added [`FileServer::identity()`], `FileServer::without_index()`, `FileServer::filter()`,
    `FileServer::map()`, `FileServer::rewrite()`, and the [rewrites] in `fs::rewrite`. ([#2716])
  * Added `Redirect::map_uri()`, `Response::set_header_map()`, `NamedFile::open_with()`, and
    `TempFile::is_empty()`.
  * Responses dispatched by local clients include a `Content-Length` header when the size of the
    body is known. Previously, the header was only set on responses sent by the server.

### Trait Implementations

  * Implemented `Clone` for `Request`, `Redirect`, and `Shield`.
  * Implemented `Default` for `TlsConfig`.
  * Implemented `IntoIterator` for `HeaderMap`, `MediaType`, `QMediaType`, and `shield::Allow`.
  * Implemented `AsRef<OsStr>` for `RawStr`, `uri::Path`, and `uri::Query`.
  * Implemented `AsRef<str>`, `TryFrom<&str>`, `TryFrom<&[u8]>`, and `PartialEq<&Method>` for
    `Method`.
  * Implemented `PartialEq<&T>` for `T` and for `Uri`, where `T` is any URI type.
  * Implemented `std::error::Error` for `uri::Error` and `uri::error::PathError`.
  * Implemented `Display` for `uri::error::PathError`.
  * Implemented `UriDisplay` and `FromUriParam` for `Either`.
  * Implemented `From<figment::Error>` and `From<io::Error>` for `Error`.

### Contrib Libraries

  * Added the `diesel_sqlite` feature and [`diesel::SqlitePool`] to `rocket_db_pools`.
  * `rocket_db_pools` reexports Diesel's `SyncConnectionWrapper` and `AsyncConnectionWrapper`.
  * `rocket_sync_db_pools` rejects a Memcache configuration without a server URL. Previously, the
    pool's clients panicked on first use.

### Updated Dependencies

  * The `tracing` dependency was introduced (`0.1`).
  * The `tracing-subscriber` dependency was introduced (`0.3`).
  * The `tinyvec` dependency was introduced (`1.6`).
  * The `thread_local` dependency was introduced (`1.1`).
  * The `quick-xml` dependency was introduced (`0.42`).
  * The `quinn` dependency was introduced (`0.11`).
  * The `h3` dependency was introduced (`0.0.8`).
  * The `h3-quinn` dependency was introduced (`0.0.10`).
  * The `ring` dependency was introduced (`0.17.12`).
  * The `libc` dependency was introduced on Unix (`0.2`).
  * The `percent-encoding` dependency was introduced in `rocket_codegen` (`2.3`).
  * The `log` dependency was introduced in `rocket_db_pools` (`0.4`).
  * The `http` dependency was updated to `1.1`.
  * The `x509-parser` dependency was updated to `0.18`.
  * The `parking_lot` dependency was updated to `0.13`.
  * The minimum `rusqlite` and `r2d2_sqlite` versions for `rocket_sync_db_pools` are now `0.42`
    and `0.37`, respectively.
  * The minimum `libsqlite3-sys` version for `rocket_db_pools` and `rocket_sync_db_pools` is now
    `0.40`.
  * The `log` dependency was removed from `rocket` and `rocket_http`.
  * The `smallvec` dependency was removed.
  * The `http-body-util` dependency was removed.
  * `rocket_http` no longer depends on `hyper`, `http`, `h2`, `tokio`, `futures`,
    `pin-project-lite`, `x509-parser`, or any TLS library. Its `tls`, `mtls`, `http2`,
    `private-cookies`, and `rustls-pemfile` features were removed.

## Infrastructure

The following changes were made to the project's infrastructure:

  * The workspace resolves and builds with the minimum allowed version of every direct dependency.
  * All crates use workspace-wide lints.
  * Added an end-to-end testbench. ([#1509])
  * Added fuzzing targets for routing collision safety and URI normalization.
  * Added a `--help` flag to `test.sh`.
  * Added a CONTRIBUTING guide.
  * Added a v0.5 to v0.6 upgrading guide.
  * Updated documentation for all changes.
  * Fixed many typos throughout documentation. ([#2989])

[connection listeners]: https://api.rocket.rs/master/rocket/listener/index.html
[Unix domain sockets]: https://api.rocket.rs/master/rocket/listener/unix/index.html
[`tracing`]: https://docs.rs/tracing/0.1/tracing/
[HTTP/3]: https://api.rocket.rs/master/rocket/listener/quic/index.html
[`FileServer`]: https://api.rocket.rs/master/rocket/fs/struct.FileServer.html
[rewrites]: https://api.rocket.rs/master/rocket/fs/rewrite/index.html
[Dynamic TLS]: https://api.rocket.rs/master/rocket/tls/trait.Resolver.html
[`Resolver`]: https://api.rocket.rs/master/rocket/tls/trait.Resolver.html
[`CryptoProvider`]: https://rocket.rs/master/guide/configuration/#crypto-providers
[Routes for any method]: https://rocket.rs/master/guide/requests/#methods
[XML]: https://api.rocket.rs/master/rocket/serde/xml/index.html
[data transformers]: https://api.rocket.rs/master/rocket/data/trait.Transform.html
[`uri!`]: https://api.rocket.rs/master/rocket/macro.uri.html
[proxy protocol header]: https://rocket.rs/master/guide/configuration/#proxied-tls
[waker]: https://rocket.rs/master/guide/configuration/#wake-interval
[`Error::report()`]: https://api.rocket.rs/master/rocket/error/struct.Error.html#method.report
[Fairing retrieval]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.fairing
[route matching]: https://api.rocket.rs/master/rocket/route/struct.Route.html#method.matches
[upgrading guide]: https://rocket.rs/master/guide/upgrading/
[`AdHoc::uri_normalizer()`]: https://api.rocket.rs/master/rocket/fairing/struct.AdHoc.html#method.uri_normalizer
[`FileServer::new()`]: https://api.rocket.rs/master/rocket/fs/struct.FileServer.html#method.new
[`MsgPack`]: https://api.rocket.rs/master/rocket/serde/msgpack/struct.MsgPack.html
[`Compact`]: https://api.rocket.rs/master/rocket/serde/msgpack/type.Compact.html
[`Error`]: https://api.rocket.rs/master/rocket/error/struct.Error.html
[`MessageStream`]: https://api.rocket.rs/master/rocket_ws/stream/struct.MessageStream.html
[`Rocket::endpoints()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.endpoints
[`Rocket::reconfigure()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.reconfigure
[`ShutdownConfig`]: https://api.rocket.rs/master/rocket/shutdown/struct.ShutdownConfig.html
[`rocket::shutdown`]: https://api.rocket.rs/master/rocket/shutdown/index.html
[`MtlsConfig`]: https://api.rocket.rs/master/rocket/mtls/struct.MtlsConfig.html
[`tls::CipherSuite`]: https://api.rocket.rs/master/rocket/tls/enum.CipherSuite.html
[`Level`]: https://api.rocket.rs/master/rocket/config/struct.Level.html
[`CliColors`]: https://api.rocket.rs/master/rocket/config/enum.CliColors.html
[`Trace::trace_error()`]: https://api.rocket.rs/master/rocket/trace/trait.Trace.html#method.trace_error
[`Trace`]: https://api.rocket.rs/master/rocket/trace/trait.Trace.html
[`ErrorKind`]: https://api.rocket.rs/master/rocket/error/enum.ErrorKind.html
[`rocket::either::Either`]: https://api.rocket.rs/master/rocket/index.html#reexport.either
[`route`]: https://api.rocket.rs/master/rocket/attr.route.html
[`Method`]: https://api.rocket.rs/master/rocket/http/enum.Method.html
[`Method::allows_request_body()`]: https://api.rocket.rs/master/rocket/http/enum.Method.html#method.allows_request_body
[`Rocket::mount()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.mount
[`Rocket::register()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.register
[`RouteUri`]: https://api.rocket.rs/master/rocket/route/struct.RouteUri.html
[`Catcher::base()`]: https://api.rocket.rs/master/rocket/catcher/struct.Catcher.html#method.base
[`Empty`]: https://api.rocket.rs/master/rocket/error/struct.Empty.html
[`Endpoint`]: https://api.rocket.rs/master/rocket/listener/enum.Endpoint.html
[`Pool`]: https://api.rocket.rs/master/rocket_db_pools/trait.Pool.html
[`InvalidOption`]: https://api.rocket.rs/master/rocket/error/struct.InvalidOption.html
[`Rocket::fairing()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.fairing
[`Rocket::launch_with()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.launch_with
[`Rocket::launch_on()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.launch_on
[`Rocket::try_launch_on()`]: https://api.rocket.rs/master/rocket/struct.Rocket.html#method.try_launch_on
[`Route::rebase()`]: https://api.rocket.rs/master/rocket/route/struct.Route.html#method.rebase
[`Catcher::rebase()`]: https://api.rocket.rs/master/rocket/catcher/struct.Catcher.html#method.rebase
[`wake_interval`]: https://api.rocket.rs/master/rocket/struct.Config.html#structfield.wake_interval
[`proxy_proto_header`]: https://api.rocket.rs/master/rocket/struct.Config.html#structfield.proxy_proto_header
[`Request::proxy_proto()`]: https://api.rocket.rs/master/rocket/request/struct.Request.html#method.proxy_proto
[`Request::context_is_likely_secure()`]: https://api.rocket.rs/master/rocket/request/struct.Request.html#method.context_is_likely_secure
[`Data::chain_transform()`]: https://api.rocket.rs/master/rocket/data/struct.Data.html#method.chain_transform
[`rocket::trace`]: https://api.rocket.rs/master/rocket/trace/index.html
[`RocketFmt`]: https://api.rocket.rs/master/rocket/trace/subscriber/struct.RocketFmt.html
[`RocketDynFmt`]: https://api.rocket.rs/master/rocket/trace/subscriber/struct.RocketDynFmt.html
[`RequestId`]: https://api.rocket.rs/master/rocket/trace/subscriber/struct.RequestId.html
[`rocket::serde::xml`]: https://api.rocket.rs/master/rocket/serde/xml/index.html
[`HttpVersion`]: https://api.rocket.rs/master/rocket/http/enum.HttpVersion.html
[`Request::version()`]: https://api.rocket.rs/master/rocket/request/struct.Request.html#method.version
[`ProxyProto`]: https://api.rocket.rs/master/rocket/http/enum.ProxyProto.html
[`Listener`]: https://api.rocket.rs/master/rocket/listener/trait.Listener.html
[`Bind`]: https://api.rocket.rs/master/rocket/listener/trait.Bind.html
[`rocket::tls`]: https://api.rocket.rs/master/rocket/tls/index.html
[`TlsListener`]: https://api.rocket.rs/master/rocket/tls/struct.TlsListener.html
[`Request::sni()`]: https://api.rocket.rs/master/rocket/request/struct.Request.html#method.sni
[`LocalRequest::xml()`]: https://api.rocket.rs/master/rocket/local/blocking/struct.LocalRequest.html#method.xml
[`LocalResponse::into_xml()`]: https://api.rocket.rs/master/rocket/local/blocking/struct.LocalResponse.html#method.into_xml
[`FileServer::identity()`]: https://api.rocket.rs/master/rocket/fs/struct.FileServer.html#method.identity
[`diesel::SqlitePool`]: https://api.rocket.rs/master/rocket_db_pools/diesel/type.SqlitePool.html
[#21]: https://github.com/rwf2/Rocket/issues/21
[#232]: https://github.com/rwf2/Rocket/issues/232
[#775]: https://github.com/rwf2/Rocket/issues/775
[#1070]: https://github.com/rwf2/Rocket/issues/1070
[#1188]: https://github.com/rwf2/Rocket/issues/1188
[#1509]: https://github.com/rwf2/Rocket/issues/1509
[#1561]: https://github.com/rwf2/Rocket/issues/1561
[#2107]: https://github.com/rwf2/Rocket/issues/2107
[#2239]: https://github.com/rwf2/Rocket/issues/2239
[#2363]: https://github.com/rwf2/Rocket/issues/2363
[#2512]: https://github.com/rwf2/Rocket/issues/2512
[#2533]: https://github.com/rwf2/Rocket/issues/2533
[#2577]: https://github.com/rwf2/Rocket/issues/2577
[#2649]: https://github.com/rwf2/Rocket/issues/2649
[#2671]: https://github.com/rwf2/Rocket/issues/2671
[#2683]: https://github.com/rwf2/Rocket/issues/2683
[#2716]: https://github.com/rwf2/Rocket/issues/2716
[#2723]: https://github.com/rwf2/Rocket/issues/2723
[#2730]: https://github.com/rwf2/Rocket/issues/2730
[#2731]: https://github.com/rwf2/Rocket/issues/2731
[#2748]: https://github.com/rwf2/Rocket/issues/2748
[#2840]: https://github.com/rwf2/Rocket/issues/2840
[#2919]: https://github.com/rwf2/Rocket/issues/2919
[#2942]: https://github.com/rwf2/Rocket/issues/2942
[#2969]: https://github.com/rwf2/Rocket/issues/2969
[#2989]: https://github.com/rwf2/Rocket/issues/2989
[#3015]: https://github.com/rwf2/Rocket/issues/3015

# Version 0.5.4 (Nov 19, 2026)

This release contains the following crate updates:

  - `rocket` `0.5.4`

## [`rocket`](https://api.rocket.rs/v0.5/rocket/) `0.5.4`

### General Improvements

  * **(fix [#3022] via [#3023])** A response body is now dropped as soon as the
    client disconnects.

    Previously, Rocket detected a disconnected client only when it sent the
    next part of the body. A body that produced no further data, for instance,
    a stream waiting on a stalled source, was retained and polled until the
    server shut down. Rocket now stops reading and drops the body when the
    HTTP/1.1 connection closes or the HTTP/2 stream is reset.

[#3022]: https://github.com/rwf2/Rocket/issues/3022
[#3023]: https://github.com/rwf2/Rocket/pull/3023

# Version 0.5.3 (Nov 18, 2026)

This release contains the following crate updates:

  - `rocket` `0.5.3`
  - `rocket_db_pools` `0.3.0`
  - `rocket_dyn_templates` `0.3.0`
  - `rocket_sync_db_pools` `0.2.0`
  - `rocket_ws` `0.2.0`

## [`rocket`](https://api.rocket.rs/v0.5/rocket/) `0.5.3`

### General Improvements

  * **(fix [#3004])** Connections now linger for up to 10 seconds after the
    last response.

    Previously, Rocket closed a connection immediately after the last response,
    even if the client was still sending data, for instance, the rest of an
    upload that exceeded a data limit. Closing a connection with unread data
    causes TCP to send a reset, which can discard the response before the
    client reads it. Rocket now closes connections in stages: it shuts down
    writes, then reads and discards incoming data until the client closes the
    connection, the linger time elapses, or the shutdown grace period ends.
    Upgraded connections, such as WebSocket connections, do not linger.

    As a result, connections may remain open for up to 10 seconds longer than
    before. The linger time is configured via the new [`linger`] configuration
    parameter. For the previous behavior, set `linger` to `0`.

  * **(fix [#3018] via [#3019])** A response whose body fails to read is now
    aborted.

    Previously, clients received the part of the body read before the error as
    a complete response. Rocket now closes the HTTP/1.1 connection or resets
    the HTTP/2 stream, so clients see an incomplete response.

  * A request body that fails to read now fails every subsequent read.

    Previously, an error that occurred while Rocket peeked into the body was
    lost, and handlers read a truncated or malformed body as complete.

  * Graceful shutdown now completes as soon as all connections close.

    Previously, Rocket waited for the entire grace period if any connection was
    open when shutdown was requested, and often for the entire mercy period as
    well.

### Dependency Changes

  * `bytes` was updated to `1.11.1`, fixing an integer overflow in
    `BytesMut::reserve()` ([RUSTSEC-2026-0007]).

  * `tempfile` was updated to `3.4.0`, which no longer depends on the vulnerable
    `remove_dir_all` `0.5` ([RUSTSEC-2023-0018]).

  * `async-stream` was updated to `0.3.3`, fixing compile errors in stream
    macros when `yield` and `?` are used in the same expression, as in
    `yield message?`.

  * `ref-swap` was removed. Version `0.1.2` is unsound ([RUSTSEC-2026-0528]).
    Only Rocket itself wrote through `ref-swap`, so the unsoundness was not
    reachable through Rocket's API.

## [`rocket_db_pools`](https://api.rocket.rs/v0.5/rocket_db_pools/) `0.3.0`

### Breaking Changes

This release includes many breaking changes, most of which are the result of
updated database drivers. These changes are listed below along with a short
note about how to handle the breaking change in existing applications.
Applications that depend on a driver directly must update the dependency to
match or use the driver reexported by `rocket_db_pools`.

#### Silent Changes

These changes are invisible to the compiler and will _not_ yield errors or
warnings at compile-time. We **strongly** advise all application authors to
review this list carefully.

  * **Redis commands time out after 0.5 seconds by default.**

    Previously, Redis commands had no response timeout. Commands that block by
    design, such as `BLPOP`, now fail if they block for longer than the
    timeout. Use the new `response_timeout` configuration option to set the
    timeout in seconds. A value of `0` disables the timeout:

    ```toml
    [default.databases.redis]
    url = "redis://localhost:6379"
    response_timeout = 0
    ```

    A timeout does not cancel a command already executing on Redis.

  * **The `extensions` configuration option is ignored.**

    Loading a SQLite extension requires an `unsafe` call in SQLx `0.9`. As
    such, `Config::extensions` was removed, and a warning is emitted if
    `extensions` is configured for `sqlx_sqlite`. To load extensions, call
    SQLx's `SqliteConnectOptions::extension()` method from a
    [`Database::custom()`] callback:

    ```rust
    rocket::build().attach(Db::custom(|_config, (pool, connection)| {
        // SAFETY: this library is trusted for the pool's lifetime.
        let connection = unsafe { connection.extension("rot13") };
        (pool, connection)
    }))
    ```

    The extension library _must_ be trusted whenever a connection is created.

#### General

The following changes are likely to cause compile-time errors.

  * **`sqlx` was updated to `0.9`.**

    Notable changes include:

    - Custom `Encode` implementations return `Result<IsNull, BoxDynError>`.
    - `HasArguments<'q>` projections were replaced with
      `<DB as Database>::ArgumentBuffer`. Argument types no longer take a
      lifetime parameter.
    - Dynamic SQL requires `sqlx::AssertSqlSafe(sql)`. This does not sanitize
      SQL.
    - The `runtime-tokio-rustls` feature was replaced with `runtime-tokio` and
      `tls-rustls-ring`.

    Regenerate offline query metadata with `cargo sqlx prepare` using SQLx CLI
    `0.9`. This update fixes protocol length overflows ([RUSTSEC-2024-0363])
    and PostgreSQL build failures on newer compilers.

  * **`mongodb` was updated to `3`.**

    Operations are now configured with builders. Remove `None` option arguments
    and pass options via `with_options()`:

    ```diff
    - collection.find_one(filter, None).await?
    + collection.find_one(filter).await?

    - collection.find_one(filter, options).await?
    + collection.find_one(filter).with_options(options).await?
    ```

    BSON `2` and `mongodb+srv` URLs remain supported.

  * **`diesel` was updated to `2.4` and `diesel-async` to `0.10`.**

    Transactions take `async` closures instead of boxed futures:

    ```diff
    - db.transaction(|mut conn| Box::pin(async move {
    + db.transaction(async |mut conn| {
          ...
    - })).await?;
    + }).await?;
    ```

    Use `diesel::define_sql_function!` in place of `diesel::sql_function!`.
    Applications using Diesel migrations should also update `diesel_migrations`
    to `2.4`.

    This update fixes PostgreSQL protocol length overflows
    ([RUSTSEC-2024-0365]) and undefined behavior when serializing or
    deserializing MySQL date/time values ([RUSTSEC-2026-0134],
    [RUSTSEC-2026-0138]).

  * **`deadpool-postgres` was updated to `0.14` and `deadpool-redis` to `0.23`,
    with `redis` `1`.**

    Postgres, Redis, and Diesel pools now use `deadpool` `0.13`. Notable changes
    include:

    - `deadpool::Runtime` methods are now free functions in `deadpool-runtime`
      `0.3`, taking the runtime as the first argument. `Runtime` is now
      non-exhaustive, and `SpawnBlockingError` is no longer reexported.
    - The error type of Diesel pools, also returned by the `Connection`
      request guard, is now `Error<PoolError<E>>`. It was previously
      `Error<BuildError<E>, PoolError<E>>`.
    - `query_async::<_, T>()` is now `query_async::<T>()`.
    - Redis iterators yield `Result`s.
    - Use `mget` and `mset`, not `get` and `set`, for multiple keys or
      key/value pairs.
    - `FromRedisValue::from_redis_value()` takes an owned `Value` and returns
      `Result<Self, ParsingError>`. Use `from_redis_value_ref()` for borrowed
      values.
    - The `Value` variants `Data`, `Bulk`, and `Status` were renamed to
      `BulkString`, `Array`, and `SimpleString`, respectively.
    - Public Redis enums are non-exhaustive.

### New Features

  * Introduced [`Database::custom()`] and [`Database::try_custom()`] for
    configuring SQLx pools and MongoDB clients during initialization. Other
    pools can support customization by implementing [`Configurable`].

  * TLS connections are now supported by `deadpool_postgres` and
    `diesel_postgres` via the new `postgres_rustls_native_certs` feature. Set
    `sslmode=require` in the database URL to require TLS.

    The server's certificate and hostname are verified using the platform's
    native root certificates. To use a private CA, set `SSL_CERT_FILE` or
    `SSL_CERT_DIR`. If no root certificates can be loaded, TLS connections fail.

### General Improvements

  * `Config::default()` now sets `max_connections` to the default number of
    workers * 4 and `connect_timeout` to `5`, matching the documented defaults.
    Both values were previously `0`. Configuration defaults are unchanged.

  * The documentation for `idle_timeout` now describes its effect for each
    driver. For Deadpool and Diesel pools, it limits the time allowed to
    recycle a connection, including health checks, and does not close idle
    connections.

### Dependency Changes

  * `rustls` was updated to `0.23.46`, `rustls-webpki` to `0.103.16`, and
    `webpki-roots` to `1.0.10`, fixing [RUSTSEC-2026-0382] and
    [RUSTSEC-2026-0381]. Distrusted intermediate certificates are rejected, and
    the affected emSign root was removed from the bundled trust store.

  * `tokio-postgres` was updated to `0.7.18` for `deadpool_postgres` and
    `diesel_postgres`, fixing denial of service from malformed rows, `hstore`
    values, and SCRAM iteration counts ([RUSTSEC-2026-0178],
    [RUSTSEC-2026-0179], [RUSTSEC-2026-0180]).

  * `sqlx` was updated to `0.9.1`, and `libsqlite3-sys` to `0.39.0` for
    `sqlx_sqlite`, bundling SQLite `3.54.0` and fixing [RUSTSEC-2026-0459].

  * `rsa` was updated to `0.10.0` for `sqlx_mysql`, fixing [RUSTSEC-2023-0071].

  * `lru` was updated to `0.18.2` for `diesel_mysql`, fixing a use-after-free
    ([RUSTSEC-2026-0253]).

## [`rocket_dyn_templates`](https://api.rocket.rs/v0.5/rocket_dyn_templates/) `0.3.0`

### Breaking Changes

This release includes many breaking changes, most of which are the result of
updated templating engines. These changes are listed below along with a short
note about how to handle the breaking change in existing applications.

#### Silent Changes

These changes are invisible to the compiler and will _not_ yield errors or
warnings at compile-time. We **strongly** advise all application authors to
review this list carefully.

  * **Template names include the file type extension.**

    A template in a file named `index.html.tera` was previously named `index`.
    It is now named `index.html`. Change the names in `Template::render()`
    calls and in templates, including in inheritance, includes, and partials:

    ```diff
    - Template::render("index", context! { .. })
    + Template::render("index.html", context! { .. })
    ```

    ```diff
    - {% extends "base" %}
    + {% extends "base.html" %}
    ```

    The names of templates registered directly with an engine are unchanged.

  * **HTML escaping is determined by template names alone.**

    Tera and MiniJinja templates with names ending in `.html`, `.htm`, `.xml`,
    `.xhtml`, or `.svg`, regardless of case, are now escaped. This includes
    templates registered in callbacks. For Tera, the extension may be followed
    by `.tera`, as in `email.html.tera`. To render a value without escaping,
    mark it as safe. To change these defaults, configure the engine's escaping
    in [`Templating::init()`].

    Previously, Tera considered a template's file path. A Tera template added
    from a file under a different name, as with
    `add_template_file(path, Some(name))`, was escaped according to the file's
    extension. It is now escaped only if `name` ends in one of the extensions
    above.

    **Note:** In Tera, an included template uses the escaping policy of the
    template that includes it. In MiniJinja, it uses its own. As such, name
    MiniJinja HTML fragments with an HTML extension: values in a fragment
    without one are not escaped, even when included in an HTML template.

  * **Tera templates use Tera 2 syntax and are checked when added.**

    - A template's helpers and the templates it references must exist when it
      is added. Register helpers in [`Templating::init()`]. A template in
      `template_dir` cannot depend on a template first added in `on_load()`;
      place templates that depend on each other together in `template_dir`.
    - Macros were replaced with components.
    - Arrays are indexed with `items[0]` in place of `items.0`.
    - Apostrophes are escaped as `&#39;`. Slashes are no longer escaped.

    See [Tera's update notes] for changes to built-in filters, tests,
    functions, and undefined values.

  * **MiniJinja templates follow MiniJinja 3 semantics.**

    - Floor division, modulo, and rounding follow Python's behavior. Ties are
      rounded to the nearest even number.
    - The `tojson` filter includes spaces after commas and colons.
    - Imported names beginning with `_` are private.

    See [MiniJinja's update notes] for further details.

  * **Handlebars partials and whitespace render differently.**

    Handlebars `6` fixes partial contexts, indentation, and whitespace handling,
    which may change rendered output. Default HTML escaping is unchanged.

#### General

The following changes are likely to cause compile-time errors.

  * **The templating fairing is now [`Templating`].**

    `Template::fairing()`, `Template::custom()`, and `Template::try_custom()`
    were removed. Use `Templating::new()` in place of `Template::fairing()`.

    `Template::custom()` callbacks ran _after_ templates were loaded. Callbacks
    that register helpers or configure engines should instead use
    `Templating::init()`, which runs _before_ templates are loaded:

    ```diff
    - .attach(Template::custom(|engines| {
    + .attach(Templating::init(|engines| {
          engines.handlebars.register_helper("wow", Box::new(wow_helper));
      }))
    ```

    Callbacks that add templates which use templates in `template_dir` should
    use `on_load()`. Use `try_init()` and `try_on_load()` in place of
    `try_custom()`. As before, callbacks are called again when templates are
    reloaded. See [`Templating`] for details on which callback to use.

  * **`tera` was updated to `2.4`.**

    Helpers now use Tera's `Value`, `Kwargs`, and `State` types, and fallible
    helpers return `TeraResult`. Extra built-ins moved to `tera-contrib`; enable
    the desired features and register them in [`Templating::init()`].

  * **`minijinja` was updated to `3.0`.**

    - `Value::from_serialize(value)` was replaced with
      `Value::from(Serde(value))`. MiniJinja's rendering methods accept
      `Into<Value>`; use `Serde(context)` for serializable contexts. Rocket's
      `Template` methods and `context!` continue to accept serializable values.
    - Calls through `Value` and custom `Object` implementations use
      `&mut State`.
    - The `loader` feature was removed; loaders are always available.
    - Whitespace settings moved to `SyntaxConfig`.

  * **`handlebars` was updated to `6.4`.**

    Public types are now non-exhaustive. `RenderContext::set_partial()` and
    `remove_partial()` were removed. Use `BlockContext::set_local_partial()` to
    configure local partials.

### Dependency Changes

  * `notify` was updated to `8.2.1`, fixing unaligned memory accesses and hangs
    when moving watched directories to the trash on Windows, watcher startup
    errors on macOS, and watch cleanup on Linux.

  * `normpath` was updated to `1.5.2`, fixing unsound path normalization on
    Windows.

## [`rocket_sync_db_pools`](https://api.rocket.rs/v0.5/rocket_sync_db_pools/) `0.2.0`

### Breaking Changes

This release includes several breaking changes, all of which are the result of
updated database drivers. These changes are listed below along with a short
note about how to handle the breaking change in existing applications.
Applications that depend on a driver directly must update the dependency to
match or use the driver reexported by `rocket_sync_db_pools`.

  * **`rusqlite` was updated to `0.41` through `0.42` and `r2d2_sqlite` to
    `0.36` through `0.37`.**

    Cargo selects the newest version compatible with the application's other
    SQLite dependencies. Applications that also use `sqlx_sqlite` should also
    update `rocket_db_pools` to `0.3`.

    As such, the version of `rusqlite` reexported by `rocket_sync_db_pools`
    depends on dependency resolution. Applications that use `rusqlite` types
    should depend on the desired version directly, for example
    `rusqlite = "0.42"`.

    Remove the `winsqlite3` feature and update direct `fallible-iterator`
    dependencies to `0.3`. `WindowAggregate::value()` takes `Option<&mut A>`
    in place of `Option<&A>`. With Rusqlite `0.42`, scalar function callbacks
    take `&mut Context`, and `Error::InvalidParameterType` replaces
    `InvalidFunctionParameterType` and `InvalidFilterParameterType`.

    Rusqlite `0.41` fixes SQL injection through savepoint names, an invalid
    deallocation in `ToSqlOutput::from_rc()`, and missing `Send` bounds on
    aggregate and window functions. Its `bundled` feature builds SQLite `3.54`
    ([RUSTSEC-2026-0459]).

  * **`memcache` was updated to `0.21`. The `r2d2_memcache` reexport was
    removed.**

    The `r2d2-memcache` dependency and feature were removed as well. Use
    [`MemcacheConnectionManager`] in place of
    `r2d2_memcache::MemcacheConnectionManager`. It returns
    `memcache::MemcacheError` directly. The driver
    now supports typed values with flags and compare-and-swap identifiers, as
    well as UDP bind addresses, and fixes ASCII write timeouts and responses
    split across socket reads.

    The `timeout` and `connect_timeout` URL options accept positive, finite
    numbers of seconds, including fractions. Invalid or out-of-range values
    abort initialization with `Error::Custom(MemcacheError::BadURL(..))`.

### Dependency Changes

  * `diesel` was updated to `2.4.0`, fixing PostgreSQL protocol length
    overflows ([RUSTSEC-2024-0365]), undefined behavior when serializing MySQL
    date/time values ([RUSTSEC-2026-0134]), and unaligned accesses in SQLite
    aggregate functions ([RUSTSEC-2026-0137]).

  * `libsqlite3-sys` was updated to `0.39.0` for `diesel_sqlite_pool` and
    `sqlite_pool`. When SQLite is bundled, it is now SQLite `3.54.0` or newer,
    which fixes [RUSTSEC-2026-0459]. By default, these features link the
    system's SQLite, whose version this update does not change.

  * `postgres` was updated to `0.19.14`, fixing denial of service from malformed
    rows, `hstore` values, and SCRAM iteration counts ([RUSTSEC-2026-0178],
    [RUSTSEC-2026-0179], [RUSTSEC-2026-0180]).

  * `tokio` was updated to `1.54.1`, fixing a soundness issue in `Interval`
    ([RUSTSEC-2026-0384]).

  * `r2d2` was updated to `0.8.9`, which no longer depends on an unsound
    `lock_api` ([RUSTSEC-2020-0070]).

## [`rocket_ws`](https://api.rocket.rs/v0.5/rocket_ws/) `0.2.0`

### Breaking Changes

This release includes several breaking changes, most of which are the result of
updating `tokio-tungstenite` to `0.30`. These changes are listed below along
with a short note about how to handle the breaking change in existing
applications.

#### Silent Changes

These changes are invisible to the compiler and will _not_ yield errors or
warnings at compile-time. We **strongly** advise all application authors to
review this list carefully.

  * **Connections allocate a larger read buffer.**

    Previously, each connection allocated a 4KiB read buffer. Connections now
    allocate an 8KiB read buffer by default. A configuration passed to
    `WebSocket::config()` replaces this default, and `Config::default()` uses a
    128KiB read buffer, so applications that configure connections may use
    considerably more memory. To keep a small buffer, set its size explicitly:

    ```rust
    let config = ws::Config::default()
        .max_message_size(Some(3 << 20))
        .read_buffer_size(8 * 1024);
    ```

    The read buffer size does not limit the size of incoming messages.

  * **The `WebSocket` request guard validates `Sec-WebSocket-Key`.**

    As required by RFC 6455, the key must be the Base64 encoding of 16 bytes.
    Requests with an invalid key are now forwarded with `400 Bad Request`.
    There is no workaround.

#### General

The following changes are likely to cause compile-time errors.

  * **Message payloads are `Utf8Bytes` and `Bytes`.**

    Text messages now use `Utf8Bytes`, while binary, Ping, and Pong messages use
    `Bytes`. Both types are reexported by `rocket_ws`. Use `.into()` when
    constructing messages from strings or vectors:

    ```diff
    - yield ws::Message::Ping(vec![b'h', b'i'])
    + yield ws::Message::Ping(vec![b'h', b'i'].into())
    ```

    For borrowed byte slices, use `Message::from(slice)` instead of
    `Message::binary(slice)`. For arrays, use `Message::from(array.as_slice())`.
    `Message::into_text()` and `Message::into_data()` return the new types as
    well.

  * **`CloseFrame` lost its lifetime parameter.**

    Its `reason` is now `Utf8Bytes`. Remove calls to `CloseFrame::into_owned()`.

  * **`Frame` payload methods were changed.**

    - `Frame::payload()` returns `&[u8]`, and `payload_mut()` was removed.
    - `Frame::into_data()` was replaced with `into_payload()`, returning
      `Bytes`.
    - `Frame::into_string()` was replaced with `into_text()`, returning
      `Result<Utf8Bytes, Utf8Error>`.
    - `Frame::from_payload()` takes `Bytes`. Convert vectors with `.into()`.

  * **`Error::WriteBufferFull` and `Error::Http` box their contents.**

    `Error::Utf8` now contains a `String`.

  * **`Config` is now non-exhaustive.**

    Use builder methods on `Config::default()` in place of struct literals:

    ```diff
    - let ws = ws.config(ws::Config {
    -     max_message_size: Some(3 << 20),
    -     ..Default::default()
    - });
    + let ws = ws.config(ws::Config::default().max_message_size(Some(3 << 20)));
    ```

    The deprecated `max_send_queue` field, which had no effect, was removed.

### General Improvements

  * `MessageStream` now sends a yielded Close frame before ending the stream,
    preserving the close code and reason. The application's stream is no longer
    polled after yielding a Close frame.

  * `MessageStream` now flushes replies to received Close frames, including
    when the application's stream ends without yielding a reply.

## General Changes

  * `rocket_db_pools`, `rocket_dyn_templates`, `rocket_sync_db_pools`, and
    `rocket_ws` require `rocket` `0.5.3`.

    Updating `rocket` alone does not update the drivers used by older database
    libraries. To receive the driver fixes listed above, update
    `rocket_db_pools` and `rocket_sync_db_pools` as well.

  * The `rust-version` for `rocket_db_pools` was updated to `1.94`, as required
    by SQLx `0.9`. The `rust-version` for the remaining contrib libraries and
    both database codegen crates was updated to `1.89`.

  * `rocket_db_pools_codegen` was updated to `0.3.0` and
    `rocket_sync_db_pools_codegen` to `0.2.0`. Each database library requires
    its matching codegen version exactly.

  * Database codegen now uses `devise` `0.5` and `proc-macro2-diagnostics`
    `0.11`. Macro diagnostics no longer contain ANSI escape sequences.

[#3004]: https://github.com/rwf2/Rocket/issues/3004
[#3018]: https://github.com/rwf2/Rocket/issues/3018
[#3019]: https://github.com/rwf2/Rocket/pull/3019
[`linger`]: https://api.rocket.rs/v0.5/rocket/struct.Config.html#structfield.linger
[`Database::custom()`]: https://api.rocket.rs/v0.5/rocket_db_pools/trait.Database.html#method.custom
[`Database::try_custom()`]: https://api.rocket.rs/v0.5/rocket_db_pools/trait.Database.html#method.try_custom
[`Configurable`]: https://api.rocket.rs/v0.5/rocket_db_pools/trait.Configurable.html
[`Templating`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/struct.Templating.html
[`Templating::init()`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/struct.Templating.html#method.init
[Tera's update notes]: https://github.com/Keats/tera/blob/v2.4.0/MIGRATION.md
[MiniJinja's update notes]: https://github.com/mitsuhiko/minijinja/blob/3.0.3/UPDATING.md
[`MemcacheConnectionManager`]: https://api.rocket.rs/v0.5/rocket_sync_db_pools/struct.MemcacheConnectionManager.html
[RUSTSEC-2020-0070]: https://rustsec.org/advisories/RUSTSEC-2020-0070.html
[RUSTSEC-2023-0018]: https://rustsec.org/advisories/RUSTSEC-2023-0018.html
[RUSTSEC-2023-0071]: https://rustsec.org/advisories/RUSTSEC-2023-0071.html
[RUSTSEC-2024-0363]: https://rustsec.org/advisories/RUSTSEC-2024-0363.html
[RUSTSEC-2024-0365]: https://rustsec.org/advisories/RUSTSEC-2024-0365.html
[RUSTSEC-2026-0007]: https://rustsec.org/advisories/RUSTSEC-2026-0007.html
[RUSTSEC-2026-0134]: https://rustsec.org/advisories/RUSTSEC-2026-0134.html
[RUSTSEC-2026-0137]: https://rustsec.org/advisories/RUSTSEC-2026-0137.html
[RUSTSEC-2026-0138]: https://rustsec.org/advisories/RUSTSEC-2026-0138.html
[RUSTSEC-2026-0178]: https://rustsec.org/advisories/RUSTSEC-2026-0178.html
[RUSTSEC-2026-0179]: https://rustsec.org/advisories/RUSTSEC-2026-0179.html
[RUSTSEC-2026-0180]: https://rustsec.org/advisories/RUSTSEC-2026-0180.html
[RUSTSEC-2026-0253]: https://rustsec.org/advisories/RUSTSEC-2026-0253.html
[RUSTSEC-2026-0381]: https://rustsec.org/advisories/RUSTSEC-2026-0381.html
[RUSTSEC-2026-0382]: https://rustsec.org/advisories/RUSTSEC-2026-0382.html
[RUSTSEC-2026-0384]: https://rustsec.org/advisories/RUSTSEC-2026-0384.html
[RUSTSEC-2026-0459]: https://rustsec.org/advisories/RUSTSEC-2026-0459.html
[RUSTSEC-2026-0528]: https://rustsec.org/advisories/RUSTSEC-2026-0528.html

# Version 0.5.2 (Oct 23, 2026)

This release contains the following crate updates:

  - `rocket` `0.5.2`

## [`rocket`](https://api.rocket.rs/v0.5/rocket/) `0.5.2`

### Security Updates

  * `rustls` was updated to `0.23.46`, `tokio-rustls` to `0.26.6`, and
    `rustls-webpki` to `0.103.16`, fixing [RUSTSEC-2026-0382]. Distrusted
    intermediate certificates are now rejected.

  * `time` was updated to `0.3.47`, fixing a stack overflow when parsing
    RFC 2822 dates with deeply nested comments ([RUSTSEC-2026-0009]).

  * `tokio` was updated to `1.54.1`, fixing a soundness issue in `Interval`
    ([RUSTSEC-2026-0384]).

  * Documentation now warns that clients can spoof `X-Real-IP` unless all
    requests go through a trusted proxy that overwrites it.

  * `hyper` was updated to `1.12`, and `h2` now requires at least `0.4.20`,
    fixing excessive memory use from empty HTTP/2 frames ([RUSTSEC-2026-0258]).

  * `atomic` was replaced with `ref-swap`, fixing undefined behavior when
    reading a request's matched route.

### Fixes and Improvements

  * **(fix [#2934] via [8ddfe06c])** Multipart fields without a `Content-Type`
    now preserve their original bytes. Binary uploads are no longer corrupted
    by text decoding.

    Fields with a file name, including an empty name, are now parsed as data.
    Fields without a file name or `Content-Type` are parsed as values if their
    contents are valid UTF-8, and as binary data otherwise.

    These data fields are subject to the receiving guard's limits: `file` or
    `file/bin`, `string`, or `bytes`. [`TempFile`] writes them to `temp_dir`,
    which must exist and be writable.

    UTF-8 byte-order marks are now preserved. Numeric fields with a leading
    byte-order mark no longer parse.

  * **(fix [#2821] via [4f8b038a])** `204` and `304` responses no longer send
    a body. `204` responses no longer send `Content-Length` or
    `Transfer-Encoding` headers.

    `HEAD` and `304` responses preserve known body sizes in `Content-Length`.
    Unknown sizes are no longer set to zero, and `Content-Length` is never
    sent with `Transfer-Encoding`.

  * **(fix [#2844] via [de4bbf60])** `Status` constants can now be used in
    `match` patterns.

  * Routes declared inside functions or macros no longer cause
    `non_local_definitions` warnings.

  * Mismatched TLS certificates and private keys now cause launch to fail.
    Mutual TLS also requires a nonempty CA certificate store, even when
    client authentication is optional.

### Internal Updates

  * The internal names generated for `uri!` macros are now deterministic.

  * `rustls-pemfile` was replaced with `rustls-pki-types` `1.15.1`.

    The PEM decoder now accepts whitespace in base64 data and skips unknown
    section types. Decoded sections are limited to 256 MiB.

  * `syn`, `devise`, and `proc-macro2-diagnostics` were updated to `3`, `0.5`,
    and `0.11`, respectively.

  * `pear` was updated to `0.3`.

  * Removed the unused `rand` dependency from `rocket`.

  * Fixed compiler warnings in core and generated code.

## General Changes

  * The `rust-version` for `rocket`, `rocket_http`, and `rocket_codegen` was
    updated to `1.89`.

    This is the minimum Rust version required by the updated dependencies.

  * The examples and guide now use `rand` `0.10`, and the `state` example uses
    `flume` `0.12`.

  * Fixed `COPY` destinations in the deployment guide's Dockerfile.

  * **(fix [#2880] via [2163da51])** `FromParam` documentation now correctly
    lists `422` as the status used when forwarding after a parse error.

[RUSTSEC-2026-0009]: https://rustsec.org/advisories/RUSTSEC-2026-0009.html
[RUSTSEC-2026-0258]: https://rustsec.org/advisories/RUSTSEC-2026-0258.html
[RUSTSEC-2026-0382]: https://rustsec.org/advisories/RUSTSEC-2026-0382.html
[RUSTSEC-2026-0384]: https://rustsec.org/advisories/RUSTSEC-2026-0384.html
[#2934]: https://github.com/rwf2/Rocket/issues/2934
[#2821]: https://github.com/rwf2/Rocket/issues/2821
[#2844]: https://github.com/rwf2/Rocket/issues/2844
[#2880]: https://github.com/rwf2/Rocket/issues/2880
[8ddfe06c]: https://github.com/rwf2/Rocket/commit/8ddfe06c
[4f8b038a]: https://github.com/rwf2/Rocket/commit/4f8b038a
[de4bbf60]: https://github.com/rwf2/Rocket/commit/de4bbf60
[2163da51]: https://github.com/rwf2/Rocket/commit/2163da51

# Version 0.5.1 (May 22, 2024)

This release contains the following crate updates:

  - `rocket` `0.5.1`
  - `rocket_db_pools` `0.2.0`
  - `rocket_dyn_templates` `0.2.0`
  - `rocket_ws` `0.1.1`

## [`rocket`](https://api.rocket.rs/v0.5/rocket/) `0.5.1`

  * The following `char` and `std::ops::Range` types now implement `FromForm`:

    - `char`
    - `Range<T: FromForm>` with `start` and `end` fields
    - `RangeFrom<T: FromForm>` with `start` field
    - `RangeTo<T: FromForm>` with `end` field
    - `RangeToInclusive<T: FromForm>` with `end` field

  * `[T; N]`, `Vec<T>`, and `[u8]` can now be passed to `uri!`.

  * The guide now includes a [deploying section].

  * The `FromForm` derive now properly records errors involving entire forms.

  * `FromForm` derive can now be used in code emitted by `macro_rules!` macros.

  * **(fix [#2668] via [52de9a])** [`TempFile`] now ensures it flushes before
    being persisted.

## [`rocket_db_pools`](https://api.rocket.rs/v0.5/rocket_db_pools/) `0.2.0`

  * SQLite extensions are now supported in `sqlx_sqlite`.

    Use a database configuration option of `extensions` to specify extensions:

    ```toml
    [default.databases.db_name]
    url = "db.sqlite"
    # This option is only supported by the `sqlx_sqlite` driver.
    extensions = ["memvfs", "rot13"]
    ```

  * (**breaking**) `deadpool` was updated to `0.13`.

  * (**breaking**) The [`Config`](https://api.rocket.rs/v0.5/rocket_db_pools/struct.Config)
    structure has a new `extensions` field.

## [`rocket_dyn_templates`](https://api.rocket.rs/v0.5/rocket_dyn_templates/) `0.2.0`

  * Support for `minijinja` `2.0` templates was introduced.

    Templates with an extension of `.j2` are recognized and rendered with
    Minijinja.

  * **(breaking)** `handlebars` was updated to `5.1`.

## [`rocket_ws`](https://api.rocket.rs/v0.5/rocket_ws/) `0.1.1`

  * Introduced [`WebSocket::accept_key()`] method.

  * `tungstenite` was updated to `0.21`.

## General Changes

  * The `rust-version` for all crates was updated to `1.64`.

    This reflects the correct MSRV required to build `rocket` `0.5.0`.

  * License files are now present in all published crates.

[52de9a]: https://github.com/rwf2/Rocket/commit/52de9a
[#2668]: https://github.com/rwf2/Rocket/pull/2668
[deploying section]: https://rocket.rs/guide/v0.5/deploying/
[`WebSocket::accept_key()`]: https://api.rocket.rs/v0.5/rocket_ws/struct.WebSocket#method.accept_key

# Version 0.5.0 (Nov 17, 2023)

## Major Features and Improvements

This release introduces the following major features and improvements:

  * Support for [compilation on Rust's stable] release channel.
  * A rewritten, fully asynchronous core with support for [`async`/`await`].
  * WebSocket support via [`rocket_ws`].
  * [Feature-complete forms support] including multipart, collections, [ad-hoc validation], and
    [context](https://rocket.rs/v0.5/guide/requests/#context).
  * [Sentinels]: automatic verification of application state at start-up to prevent runtime errors.
  * [Graceful shutdown] with configurable signaling, grace periods, [notification], and
    [shutdown fairings].
  * An entirely new, flexible and robust [configuration system] based on [Figment].
  * Typed [asynchronous streams] and [Server-Sent Events] with generator syntax.
  * Asynchronous database pooling support via [`rocket_db_pools`].
  * Support for [mutual TLS] and client [`Certificate`]s.
  * Automatic support for HTTP/2 including `h2` ALPN.
  * Graduation of `json`, `msgpack`, and `uuid` `rocket_contrib` [features into core].
  * An automatically enabled [`Shield`]: security and privacy headers for all responses.
  * Type-system enforced [incoming data limits] to mitigate memory-based DoS attacks.
  * Compile-time URI literals via a fully revamped [`uri!`] macro.
  * [Request connection upgrade APIs] with support for raw I/O with the client.
  * Full support for [UTF-8 characters] in routes and catchers.
  * Precise detection of missing managed state, databases, and templating with [sentinels].
  * Typed [build phases] with strict application-level guarantees.
  * [Ignorable segments]: wildcard route matching with no typing restrictions.
  * First-class [support for `serde`] for built-in guards and types.
  * New application launch attributes:
    [`#[launch]`](https://api.rocket.rs/v0.5/rocket/attr.launch.html) and
    [`#[rocket::main]`](https://api.rocket.rs/v0.5/rocket/attr.main.html).
  * [Default catchers] via `#[catch(default)]`, which handle _any_ status code.
  * [Catcher scoping] to narrow the scope of a catcher to a URI prefix.
  * Built-in libraries and support for [asynchronous testing].
  * A [`TempFile`] data and form guard for automatic uploading to a temporary file.
  * A [`Capped<T>`] data and form guard which enables detecting truncation due to data limits.
  * Support for dynamic and static prefixing and suffixing of route URIs in [`uri!`].
  * Support for custom config profiles and [automatic typed config extraction].
  * Rewritten, zero-copy, RFC compliant URI parsers with support for URI-[`Reference`]s.
  * Multi-segment parameters (`<param..>`) which match _zero_ segments.
  * A [`local_cache!`] macro for request-local storage of non-uniquely typed values.
  * A [`CookieJar`] without "one-at-a-time" limitations.
  * [Singleton fairings] with replacement and guaranteed uniqueness.
  * [Data limit declaration in SI units]: "2 MiB", `2.mebibytes()`.
  * Optimistic responding even when data is left unread or limits are exceeded.
  * Fully decoded borrowed strings as dynamic parameters, form and data guards.
  * Borrowed byte slices as data and form guards.
  * Fail-fast behavior for [misconfigured secrets], file serving paths.
  * Support for generics and custom generic bounds in
    [`#[derive(Responder)]`](https://api.rocket.rs/v0.5/rocket/derive.Responder.html).
  * [Default ranking colors], which prevent more routing collisions automatically.
  * Improved error logging with suggestions when common errors are detected.
  * Completely rewritten examples including a new real-time [`chat`] application.

## Support for Rust Stable

As a result of support for Rust stable (Rust 2021 Edition and beyond),
`#![feature(..)]` crate attributes are no longer required to use Rocket. The
complete canonical example with a single `hello` route becomes:

```rust
#[macro_use] extern crate rocket;

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello])
}
```

<details>
  <summary>See a <code>diff</code> of the changes from v0.4.</summary>

```diff
- #![feature(proc_macro_hygiene, decl_macro)]
-
 #[macro_use] extern crate rocket;

 #[get("/<name>/<age>")]
- fn hello(name: String, age: u8) -> String {
+ fn hello(name: &str, age: u8) -> String {
     format!("Hello, {} year old named {}!", age, name)
}

- fn main() {
-     rocket::ignite().mount("/hello", routes![hello]).launch();
- }
+ #[launch]
+ fn rocket() -> _ {
+     rocket::build().mount("/hello", routes![hello])
+ }
```

</details>

## Breaking Changes

This release includes many breaking changes. For a walkthrough guide on handling these changes, see
the [v0.4 to v0.5 migration guide]. The most significant changes are listed below.

### Silent Changes

These changes are invisible to the compiler and will _not_ yield errors or warnings at compile-time.
We **strongly** advise all application authors to review this list carefully.

  * Blocking I/O (long running compute, synchronous `sleep()`, `Mutex`, `RwLock`, etc.) may prevent
    the server from making progress and should be avoided, replaced with an `async` variant, or
    performed in a worker thread. This is a consequence of Rust's cooperative `async` multitasking.
    For details, see the new [multitasking] section of the guide.
  * `ROCKET_ENV` is now `ROCKET_PROFILE`. A warning is emitted a launch time if the former is set.
  * The default profile for debug builds is now `debug`, not `dev`.
  * The default profile for release builds is now `release`, not `prod`.
  * `ROCKET_LOG` is now `ROCKET_LOG_LEVEL`. A warning is emitted a launch time if the former is set.
  * `ROCKET_ADDRESS` accepts only IP addresses, no longer resolves hostnames like `localhost`.
  * `ROCKET_CLI_COLORS` accepts booleans `true`, `false` in place of strings `"on"`, `"off"`.
  * It is a launch-time error if `secrets` is enabled in non-`debug` profiles without a configured
    `secret_key`.
  * A misconfigured `template_dir` is reported as an error at launch time.
  * [`FileServer::new()`] fails immediately if the provided directory does not exist.
  * Catcher collisions result in a launch failure as opposed to a warning.
  * Default ranks now range from `-12` to `-1`. There is no breaking change if only code generated
    routes are used. Manually configured routes with negative ranks may collide or be considered in
    a different order than before.
  * The order of execution of path and query guards relative to each other is now unspecified.
  * URIs beginning with `:` are properly recognized as invalid and rejected.
  * URI normalization now normalizes the query part as well.
  * The `Segments` iterator now returns percent-decoded `&str`s.
  * Forms are now parsed leniently by the [`Form` guard]. Use [`Strict`] for the previous behavior.
  * The `Option<T>` form guard defaults to `None` instead of the default value for `T`.
  * When data limits are exceeded, a `413 Payload Too Large` status is returned to the client.
  * The default catcher now returns JSON when the client indicates preference via the `Accept`
    header.
  * Empty boolean form values parse as `true`: the query string `?f` is the same as `?f=true`.
  * [`Created<R>`] does not automatically send an `ETag` header if `R: Hash`. Use
    [`Created::tagged_body`] instead.
  * `FileServer` now forwards when a file is not found instead of failing with `404 Not Found`.
  * [`Shield`] is enabled by default. You may need to disable or change policies if your application
    depends on typically insecure browser features or if you wish to opt-in to different policies
    than the defaults.
  * [`CookieJar`] `get()`s do not return cookies added during request handling. See
    [`CookieJar`#pending].
  * `Hash` `impl`s for `MediaType` and `ContentType` no longer consider media type parameters.
  * When requested, the `FromForm` implementations of `Vec` and `Map`s are now properly lenient.
  * To agree with browsers, the `[` and `]` characters are now accepted in URI paths.
  * The `[` and `]` characters are no longer encoded by [`uri!`].
  * The `Secure` cookie flag is set by default for all cookies when serving over TLS.
  * Removal cookies have `SameSite` set to `Lax` by default.
  * [`MediaType::JavaScript`] is now `text/javascript`.

### Contrib Graduation

The `rocket_contrib` crate is deprecated and the functionality moved to other `rocket` crates. The
[contrib deprecation upgrade guide] provides a walkthrough on migrating. The relevant changes are:

  * Several features previously in `rocket_contrib` were merged into `rocket` itself:
    - `json`, `msgpack`, and `uuid` are now [features of `rocket`].
    - Moved `rocket_contrib::json` to [`rocket::serde::json`].
    - Moved `rocket_contrib::msgpack` to [`rocket::serde::msgpack`].
    - Moved `rocket_contrib::uuid` to [`rocket::serde::uuid`].
    - Moved `rocket_contrib::helmet` to [`rocket::shield`]. [`Shield`] is enabled by default.
    - Moved `rocket_contrib::serve` to [`rocket::fs`], `StaticFiles` to [`rocket::fs::FileServer`].
    - Removed the now unnecessary `Uuid` and `JsonValue` wrapper types.
    - Removed headers in `Shield` that are no longer respected by browsers.
  * The remaining features from `rocket_contrib` are now provided by separate crates:
    - Replaced `rocket_contrib::templates` with [`rocket_dyn_templates`].
    - Replaced `rocket_contrib::databases` with [`rocket_sync_db_pools`] and [`rocket_db_pools`].
    - These crates are versioned and released independently of `rocket`.
    - `rocket_contrib::databases::DbError` is now `rocket_sync_db_pools::Error`.
    - Removed `redis`, `mongodb`, and `mysql` integrations which have upstream `async` drivers.
    - The [`#[database]`](https://api.rocket.rs/v0.5/rocket_sync_db_pools/attr.database.html)
      attribute generates an [`async run()`] method instead of `Deref` implementations.

### General

The following breaking changes apply broadly and are likely to cause compile-time errors.

  * [`Rocket`] is now generic over a [phase] marker:
    - APIs operate on `Rocket<Build>`, `Rocket<Ignite>`, `Rocket<Orbit>`, or `Rocket<P: Phase>` as
      needed.
    - The phase marker statically enforces state transitions in `Build`, `Ignite`, `Orbit` order.
    - `rocket::ignite()` is now [`rocket::build()`] and returns a `Rocket<Build>`.
    - [`Rocket::ignite()`] transitions to the `Ignite` phase. This is run automatically on launch as
      needed.
    - Ignition finalizes configuration, runs `ignite` fairings, and verifies [sentinels].
    - [`Rocket::launch()`] transitions into the `Orbit` phase and starts the server.
    - Methods like [`Request::rocket()`] that refer to a live Rocket instance return an
      `&Rocket<Orbit>`.
  * [Fairings] have been reorganized and restructured for `async`:
    - Replaced `attach` fairings with `ignite` fairings. Unlike `attach` fairings, which ran
      immediately at the time of attachment, `ignite` fairings are run when transitioning into the
      `Ignite` phase.
    - Replaced `launch` fairings with `liftoff` fairings. `liftoff` fairings are always run, even in
      local clients, after the server begins listening and the concrete port is known.
  * Introduced a new [configuration system] based on [Figment]:
    - The concept of "environments" is replaced with "profiles".
    - `ROCKET_ENV` is superseded by `ROCKET_PROFILE`.
    - `ROCKET_LOG` is superseded by `ROCKET_LOG_LEVEL`.
    - Profile names can now be arbitrarily chosen. The `dev`, `stage`, and `prod` profiles carry no
      special meaning.
    - The `debug` and `release` profiles are the default profiles for the debug and release
      compilation profiles.
    - A new specially recognized `default` profile specifies defaults for all profiles.
    - The `global` profile has highest precedence, followed by the selected profile, followed by
      `default`.
    - Added support for limits specified in SI units: "1 MiB".
    - Renamed `LoggingLevel` to [`LogLevel`].
    - Inlined error variants into the [`Error`] structure.
    - Changed the type of `workers` to `usize` from `u16`.
    - Changed accepted values for `keep_alive`: it is disabled with `0`, not `false` or `off`.
    - Disabled the `secrets` feature (for private cookies) by default.
    - Removed APIs related to "extras". Typed values can be extracted from the configured `Figment`.
    - Removed `ConfigBuilder`: all fields of [`Config`] are public with constructors for each field
      type.
  * Many functions, traits, and trait bounds have been modified for `async`:
    - [`FromRequest`], [`Fairing`], [`catcher::Handler`], [`route::Handler`], and [`FromData`] use
      `#[async_trait]`.
    - [`NamedFile::open`] is now an `async` function.
    - Added [`Request::local_cache_async()`] for use in async request guards.
    - Unsized `Response` bodies must be [`AsyncRead`] instead of `Read`.
    - Automatically sized `Response` bodies must be [`AsyncSeek`] instead of `Seek`.
    - The `local` module is split into two: [`rocket::local::asynchronous`] and
      [`rocket::local::blocking`].
  * Functionality and features requiring Rust nightly were removed:
    - Removed the `Try` implementation on [`Outcome`] which allowed using `?` with `Outcome`s. The
      recommended replacement is the [`rocket::outcome::try_outcome!`] macro or the various
      combinator functions on `Outcome`.
    - [`Result<T, E>` implements `Responder`] only when both `T` and `E` implement `Responder`. The
      new [`Debug`] wrapping responder replaces `Result<T: Responder, E: Debug>`.
    - APIs which used the `!` type to now use [`std::convert::Infallible`].
  * [`IntoOutcome`] was overhauled to supplant methods now removed in `Outcome`.
    - `IntoOutcome::into_outcome()` is now `or_error()`.
    - `IntoOutcome` is implemented for all `Outcome` type aliases.
    - `Outcome::forward()` requires specifying a status code.
    - `Outcome::from()` and `Outcome::from_or_forward()` were removed.
  * [`Rocket::register()`] now takes a base path to scope catchers under as its first argument.
  * `ErrorKind::Collision` has been renamed to [`ErrorKind::Collisions`].
  * TLS config values are only available when the `tls` feature is enabled.
  * [`MediaType::with_params()`] and [`ContentType::with_params()`] are now builder methods.
  * Content-Type [`content`] responder type names are now prefixed with `Raw`.
  * The `content::Plain` responder is now called `content::RawText`.
  * The `content::Custom<T>` responder was removed in favor of [`(ContentType, T)`].
  * Removed `CookieJar::get_private_pending()` in favor of [`CookieJar::get_pending()`].
  * The [`local_cache!`] macro accepts fewer types. Use [`local_cache_once!`] as appropriate.
  * [`Rocket::launch()`] allows `Rocket` recovery by returning the instance after shutdown.
  * `ErrorKind::Runtime` was removed; [`ErrorKind::Shutdown`] was added.
  * `Outcome::Failure` was renamed to [`Outcome::Error`].

### Routing and URIs

  * In `#[route(GET, path = "...")]`, `path` is now `uri`: `#[route(GET, uri = "...")]`.
  * Multi-segment paths (`/<p..>`) now match _zero_ or more segments.
  * Codegen improvements preclude identically named routes and modules in the same namespace.
  * A route URI like (`/<a>/<p..>`) now collides with (`/<a>`), requires a `rank` to resolve.
  * All catcher related types and traits moved to [`rocket::catcher`].
  * All route related types and traits moved to [`rocket::route`].
  * URI formatting types and traits moved to [`rocket::http::uri::fmt`].
  * `T` no longer converts to `Option<T>` or `Result<T, _>` for [`uri!`] query parameters.
  * For optional query parameters, [`uri!`] requires using a wrapped value or `_`.
  * `&RawStr` no longer implements `FromParam`: use `&str` instead.
  * Percent-decoding is performed before calling `FromParam` implementations.
  * `RawStr::url_decode()` and `RawStr::url_decode_lossy()` allocate as necessary, return `Cow`.
  * `RawStr::from_str()` was replaced with `RawStr::new()`.
  * `Origin::segments()` was replaced with `Origin.path().segments()`.
  * `Origin::path()` and `Origin::query()` return `&RawStr` instead of `&str`.
  * The type of `Route::name` is now `Option<Cow<'static, str>>`.
  * `Route::set_uri` was replaced with [`Route::map_base()`].
  * The `Route::uri` field is now of type [`RouteUri`].
  * `Route::base` was removed in favor of `Route.uri().base()`.
  * [Route `Forward` outcomes] are now associated with a `Status`.
  * The status codes used when built-in guards forward were changed:
    - Route parameter `FromParam` errors now forward as 422.
    - Query parameter errors now forward as 422.
    - Incorrect form content-type errors forwards as 413.
    - `&Host`, `&Accept`, `&ContentType`, `IpAddr`, and `SocketAddr` all forward
      with a 500.

### Data and Forms

  * `Data` now has a lifetime generic: `Data<'r>`.
  * [`Data::open()`] indelibly requires a data limit.
  * Removed `FromDataSimple`. Use [`FromData`] and [`local_cache!`] or [`local_cache_once!`].
  * All [`DataStream`] APIs require limits and return [`Capped<T>`] types.
  * Form types and traits were moved from `rocket::request` to [`rocket::form`].
  * Removed `FromQuery`. Dynamic query parameters (`#[get("/?<param>")]`) use [`FromForm`] instead.
  * Replaced `FromFormValue` with [`FromFormField`]. All `T: FromFormField` implement `FromForm`.
  * Form field values are percent-decoded before calling [`FromFormField`] implementations.
  * Renamed the `#[form(field = ...)]` attribute to `#[field(name = ...)]`.
  * [Custom form errors] must now specify an associated `Status`.

### Request Guards

  * Renamed `Cookies` to [`CookieJar`]. Its methods take `&self`.
  * Renamed `Flash.name` to `Flash.kind`, `Flash.msg` to `Flash.message`.
  * Replaced `Request::get_param()` with `Request::param()`.
  * Replaced `Request::get_segments()` to `Request::segments()`.
  * Replaced `Request::get_query_value()` with `Request::query_value()`.
  * Replaced `Segments::into_path_buf()` with `Segments::to_path_buf()`.
  * Replaced `Segments` and `QuerySegments` with [`Segments<Path>` and `Segments<Query>`].
  * [`Flash`] constructors now take `Into<String>` instead of `AsRef<str>`.
  * The `State<'_, T>` request guard is now `&State<T>`.
  * Removed a lifetime from [`FromRequest`]: `FromRequest<'r>`.
  * Removed a lifetime from [`FlashMessage`]: `FlashMessage<'_>`.
  * Removed all `State` reexports except [`rocket::State`].

### Responders

  * Moved `NamedFile` to `rocket::fs::NamedFile`
  * Replaced `Content` with `content::Custom`.
  * `Response::body` and `Response::body_mut` are now infallible methods.
  * Renamed `ResponseBuilder` to `Builder`.
  * Removed direct `Response` body reading methods. Use methods on `r.body_mut()` instead.
  * Removed inaccurate "chunked body" types and variants.
  * Removed `Responder` `impl` for `Response`. Prefer custom responders with `#[derive(Responder)]`.
  * Removed the unused reason phrase from `Status`.
  * The types of responders in [`response::status`] were unified to all be of
    the form `Status<R>(R)`.

## General Improvements

In addition to new features and changes, Rocket saw the following improvements:

### General

  * Added support for [raw identifiers] in the `FromForm` derive, `#[route]` macros, and `uri!`.
  * Added support for uncased derived form fields: `#[field(name = uncased(...))]`.
  * Added support for [default form field values]: `#[field(default = expr())]`.
  * Added support for multiple `#[field]` attributes on struct fields.
  * Added support for base16-encoded (a.k.a. hex-encoded) secret keys.
  * Added [`Config::ident`] for configuring or removing the global `Server` header.
  * Added [`Rocket::figment()`] and [`Rocket::catchers()`].
  * Added [`LocalRequest::json()`] and [`LocalResponse::json()`].
  * Added [`LocalRequest::msgpack()`] and [`LocalResponse::msgpack()`].
  * Added support for `use m::route; routes![route]` instead of needing `routes![m::route]`.
  * Added support for [hierarchical data limits]: a limit of `a/b/c` falls back to `a/b` then `a`.
  * Added [`LocalRequest::inner_mut()`]. `LocalRequest` implements `DerefMut` to `Request`.
  * Added support for ECDSA and EdDSA TLS keys.
  * Added associated constants in `Config` for all config parameter names.
  * Added `ErrorKind::Config` to represent errors in configuration at runtime.
  * Added `rocket::fairing::Result` type alias, returned by `Fairing::on_ignite()`.
  * All guard failures are logged at runtime.
  * `Rocket::mount()` now accepts a base value of any type that implements `TryInto<Origin<'_>>`.
  * The default error catcher's HTML has been compacted.
  * The default error catcher returns JSON if requested by the client.
  * Panics in routes or catchers are caught and forwarded to `500` error catcher.
  * A detailed warning is emitted if a route or catcher panics.
  * Emoji characters are no longer output on Windows.
  * Fixed [`Error`] to not panic if a panic is already in progress.
  * Introduced [`Reference`] and [`Asterisk`] URI types.
  * Added support to [`UriDisplayQuery`] for C-like enums.
  * The [`UriDisplayQuery`] derive now recognizes the `#[field]` attribute for field renaming.
  * `Client` method builders accept `TryInto<Origin>` allowing a `uri!()` to be used directly.
  * [`Rocket`] is now `#[must_use]`.
  * Support for HTTP/2 can be disabled by disabling the default `http2` crate feature.
  * Added [`rocket::execute()`] for executing Rocket's `launch()` future.
  * Added the [`context!`] macro to [`rocket_dyn_templates`] for ad-hoc template contexts.
  * The `time` crate is re-exported from the crate root.
  * The `FromForm`, `Responder`, and `UriDisplay` derives now fully support generics.
  * Added helper functions to `serde` submodules.
  * The [`Shield`] HSTS preload header now includes `includeSubdomains`.
  * Logging ignores `write!` errors if `stdout` disappears, preventing panics.
  * Added [`Client::terminate()`] to run graceful shutdown in testing.
  * Shutdown now terminates the `async` runtime, never the process.
  * Added a [`local_cache_once!`] macro for request-local storage.
  * Final launch messages are now _always_ logged, irrespective of profile.
  * Only functions that return `Rocket<Build>` are now `#[must_use]`, not all `Rocket<P>`.
  * Fixed mismatched form field names in errors under certain conditions in [`FromForm`] derive.
  * The [`FromForm`] derive now collects _all_ errors that occur.
  * Data pools are now gracefully shutdown in [`rocket_sync_db_pools`].
  * Added [`Metadata::render()`] in [`rocket_dyn_templates`] for direct template rendering.
  * Rocket salvages more information from malformed requests for error catchers.
  * The `cookie` `secure` feature is now properly conditionally enabled.
  * Data before encapsulation boundaries in TLS keys is allowed and ignored.
  * Support for TLS keys in SEC1 format was added.
  * Rocket now warns when a known secret key is configured.
  * A panic that could occur on shutdown in `rocket_sync_db_pools` was fixed.
  * Added a [`max_blocking`] configuration parameter to control number of blocking threads.
  * Added an [`ip_header`] "real IP" header configuration parameter.
  * A [`pool()`] method is emitted by [`rocket_sync_db_pools`] for code-generated pools.
  * Data guards are now eligible [sentinels].
  * Raw binary form field data can be retrieved using the `&[u8]` form guard.
  * Added [`TempFile::open()`] to stream `TempFile` data.
  * mTLS certificates can be set on local requests with [`LocalRequest::identity()`].
  * Added [`Error::pretty_print()`] for pretty-printing errors like Rocket.
  * Warnings are logged when data limits are reached.
  * A warning is emitted when `String` is used as a route parameter.
  * Configuration provenance information is logged under the `debug` log level.
  * Logging of `Outcome`s now includes the relevant status code.
  * `Span::mixed_site()` is used in codegen to reduce errant `clippy` warnings.

### HTTP

  * Added support for HTTP/2, enabled by default via the `http2` crate feature.
  * Added a `const` constructor for `MediaType`.
  * Introduced [`RawStrBuf`], an owned `RawStr`.
  * Added many new "pattern" methods to [`RawStr`].
  * Added [`RawStr::percent_encode()`] and [`RawStr::strip()`].
  * Added support for unencoded query characters in URIs that are frequently sent by browsers.
  * Introduced [`Host`] and [`&Host`] request guards.
  * Added [`RawStr::percent_encode_bytes()`].
  * `NODELAY` is now enabled on all connections by default.
  * The TLS implementation handles handshakes off the main task, improving DoS resistance.

### Known Media Types

  * Added AVIF: `image/avif`.
  * Added `EventStream`: `text/event-stream`.
  * Added `Markdown`: `text/markdown`.
  * Added `MP3`: `audio/mpeg`.
  * Added `CBZ`: `application/vnd.comicbook+zip`, extension `.cbz`.
  * Added `CBR`: `application/vnd.comicbook-rar`, extension `.cbr`.
  * Added `RAR`: `application/vnd.rar`, extension `.rar`.
  * Added `EPUB`: `application/epub+zip`, extension `.epub`.
  * Added `OPF`: `application/oebps-package+xml`, extension `.opf`.
  * Added `XHTML`: `application/xhtml+xml`, extension `.xhtml`.
  * Added `Text` as an alias for the `Plain` media type.
  * Added `Bytes` as an alias for the `Binary` media type.
  * Added `.mjs` as known JavaScript extension.
  * Added '.exe', '.iso', '.dmg' as known extensions.

### Request

  * Added support for all UTF-8 characters in route paths.
  * Added support for percent-encoded `:` in socket or IP address values in [`FromFormValue`].
  * Added [`Request::rocket()`] to access the active `Rocket` instance.
  * `Request::uri()` now returns an `&Origin<'r>` instead of `&Origin<'_>`.
  * `Request::accept()`, `Request::content_type()` reflect changes to `Accept`, `Content-Type`.
  * `Json<T>`, `MsgPack<T>` accept `T: Deserialize`, not only `T: DeserializeOwned`.
  * Diesel SQLite connections in `rocket_sync_db_pools` use better defaults.
  * The default number of workers for synchronous database pools is now `workers * 4`.
  * Added [`Request::host()`] to retrieve the client-requested host.

### Response

  * Added [`Template::try_custom()`] for fallible template engine customization.
  * Manually registered templates can now be rendered with `Template::render()`.
  * Added support for the `X-DNS-Prefetch-Control` header to `Shield`.
  * Added support for manually-set `expires` values for private cookies.
  * Added support for type generics and custom generic bounds to
    [`#[derive(Responder)]`](https://api.rocket.rs/v0.5/rocket/derive.Responder.html).
  * The `Server` header is only set if one isn't already set.
  * Accurate `Content-Length` headers are sent even for partially read `Body`s.
  * [`Redirect`] now accepts a `TryFrom<Reference>`, allowing fragment parts.

### Trait Implementations

  * Implemented `Clone` for `State`.
  * Implemented `Copy` and `Clone` for `fairing::Info`.
  * Implemented `Debug` for `Rocket` and `Client`.
  * Implemented `Default` for `Status` (returns `Status::Ok`).
  * Implemented `PartialEq`, `Eq`, `Hash`, `PartialOrd`, and `Ord` for `Status`.
  * Implemented `Eq`, `Hash`, and `PartialEq<&str>` for `Origin`.
  * Implemented `PartialEq<Cow<'_, RawStr>>>` for `RawStr`.
  * Implemented `std::error::Error` for `Error`.
  * Implemented `Deref` and `DerefMut` for `LocalRequest` (to `Request`).
  * Implemented `DerefMut` for `Form`, `LenientForm`.
  * Implemented `From<T>` for `Json<T>`, `MsgPack<T>`.
  * Implemented `TryFrom<String>` and `TryFrom<&str>` for `Origin`.
  * Implemented `TryFrom<Uri>` for each of the specific URI variants.
  * Implemented `FromRequest` for `&Config`.
  * Implemented `FromRequest` for `IpAddr`.
  * Implemented `FromParam` for `PathBuf`
  * Implemented `FromParam`, `FromData`, and `FromForm` for `&str`.
  * Implemented `FromForm` for `Json<T>`, `MsgPack<T>`.
  * Implemented `FromFormField` for `Cow` and `Capped<Cow>>`
  * Implemented `Responder` for `tokio::fs::File`.
  * Implemented `Responder` for `(ContentType, R) where R: Responder`.
  * Implemented `Responder` for `(Status, R) where R: Responder` which overrides `R`'s status.
  * Implemented `Responder` for `std::io::Error` (behaves as `Debug<std::io::Error>`).
  * Implemented `Responder` for `Either<T, E>`, equivalently to `Result<T, E>`.
  * Implemented `Serialize` for `Flash`.
  * Implemented `Serialize`, `Deserialize`, `UriDisplay` and `FromUriParam` for `uuid::Uuid`
  * Implemented `Serialize`, `Deserialize` for `RawStr`.
  * Implemented `Serialize`, `Deserialize` for all URI types.
  * Implemented `Responder` for `Arc<T>`, `Box<T>` where `T: Responder`.
  * Implemented `Serialize` and `Deserialize` for [`Method`].
  * Implemented `Eq` for [`MediaType`] and [`ContentType`].
  * Implemented `Responder` for `Box<T: Responder + Sized>`.
  * Implemented `FromForm` for `Arc<T>`.
  * Implemented `Fairing` for `Arc<dyn Fairing>`.
  * Implemented `Serialize` and `Deserialize` for `Status`.

### Dependency Changes

  * `serde` was introduced (`1.0`).
  * `futures` was introduced (`0.3`).
  * `binascii` was introduced (`0.1`).
  * `ref-cast` was introduced (`1.0`).
  * `atomic` was introduced (`0.5`).
  * `parking_lot` was introduced (`0.11`).
  * `ubtye` was introduced (`0.10`).
  * `figment` was introduced (`0.10`).
  * `rand` was introduced (`0.8`).
  * `either` was introduced (`1.0`).
  * `pin-project-lite` was introduced (`0.2`).
  * `indexmap` was introduced (`2.0`).
  * `tempfile` was introduced (`3.0`).
  * `async-trait` was introduced (`0.1`).
  * `async-stream` was introduced (`0.3`).
  * `multer` was introduced (`2.0`).
  * `tokio` was introduced (`1.6.1`).
  * `tokio-util` was introduced (`0.6`).
  * `tokio-stream` was introduced (`0.1.6`).
  * `bytes` was introduced (`1.0`).
  * `normpath` was introduced (`1`).
  * `state` was updated to `0.6`.
  * `rmp-serde` was updated to `0.15`.
  * `uuid` was updated to `0.8`.
  * `tera` was updated to `1.10`.
  * `postgres` was updated to `0.19`.
  * `rusqlite` was updated to `0.25`.
  * `r2d2_sqlite` was updated to `0.18`.
  * `time` was updated to `0.3`.
  * `handlebars` was updated to `4.0`.
  * `memcache` was updated to `0.16`.
  * `rustls` was updated to `0.21`.
  * `tokio-rustls` was updated to `0.24`.
  * `syn` was updated to `2`.
  * `diesel` was updated to `2.0`.
  * `sqlx` was updated to `0.7`.
  * `notify` was updated to `6`.
  * `criterion` was updated to `0.4`.
  * `cookie` was updated to `0.18`.
  * `yansi` was updated to `1.0`.
  * `atty` was removed.

## Infrastructure

The following changes were made to the project's infrastructure:

  * Rocket now uses the 2021 edition of Rust.
  * Added a [v0.4 to v0.5 migration guide] and [FAQ] to Rocket's website.
  * Added visible `use` statements to examples in the guide.
  * Split examples into a separate workspace for easier testing.
  * Updated documentation for all changes.
  * Fixed many typos, errors, and broken links throughout documentation and examples.
  * Improved the general robustness of macros, and the quality and frequency of error messages.
  * Benchmarks now use `criterion` and datasets extracted from real-world projects.
  * Fixed the SPDX license expressions in `Cargo.toml` files.
  * Added support to `test.sh` for a `+` flag (e.g. `+stable`) to pass to `cargo`.
  * Added support to `test.sh` for extra flags to be passed on to `cargo`.
  * UI tests are now allowed to fail by the CI to avoid false negatives.
  * The GitHub CI workflow was updated to use maintained actions.
  * The CI now frees disk space before proceeding to avoid out-of-disk errors.
  * All workspaces now use `resolver = 2`.

[phase]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#phases
[`async`/`await`]: https://rocket.rs/v0.5/guide/overview/#async-routes
[compilation on Rust's stable]: https://rocket.rs/v0.5/guide/getting-started/#installing-rust
[Feature-complete forms support]: https://rocket.rs/v0.5/guide/requests/#forms
[configuration system]: https://rocket.rs/v0.5/guide/configuration/#configuration
[graceful shutdown]: https://api.rocket.rs/v0.5/rocket/config/struct.Shutdown.html#summary
[asynchronous testing]: https://rocket.rs/v0.5/guide/testing/#asynchronous-testing
[UTF-8 characters]: https://rocket.rs/v0.5/guide/requests/#static-parameters
[ignorable segments]: https://rocket.rs/v0.5/guide/requests/#ignored-segments
[Catcher scoping]: https://rocket.rs/v0.5/guide/requests/#scoping
[ad-hoc validation]: https://rocket.rs/v0.5/guide/requests#ad-hoc-validation
[incoming data limits]: https://rocket.rs/v0.5/guide/requests/#streaming
[build phases]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#phases
[Singleton fairings]: https://api.rocket.rs/v0.5/rocket/fairing/trait.Fairing.html#singletons
[features into core]: https://api.rocket.rs/v0.5/rocket/index.html#features
[features of `rocket`]: https://api.rocket.rs/v0.5/rocket/index.html#features
[Data limit declaration in SI units]: https://api.rocket.rs/v0.5/rocket/data/struct.ByteUnit.html
[support for `serde`]: https://api.rocket.rs/v0.5/rocket/serde/index.html
[automatic typed config extraction]: https://api.rocket.rs/v0.5/rocket/fairing/struct.AdHoc.html#method.config
[misconfigured secrets]: https://api.rocket.rs/v0.5/rocket/config/struct.SecretKey.html
[default ranking colors]: https://rocket.rs/v0.5/guide/requests/#default-ranking
[`chat`]: https://github.com/rwf2/Rocket/tree/v0.5/examples/chat
[`Form` guard]: https://api.rocket.rs/v0.5/rocket/form/struct.Form.html
[`Strict`]: https://api.rocket.rs/v0.5/rocket/form/struct.Strict.html
[`CookieJar`#pending]: https://api.rocket.rs/v0.5/rocket/http/struct.CookieJar.html#pending
[`rocket::serde::json`]: https://api.rocket.rs/v0.5/rocket/serde/json/index.html
[`rocket::serde::msgpack`]: https://api.rocket.rs/v0.5/rocket/serde/msgpack/index.html
[`rocket::serde::uuid`]: https://api.rocket.rs/v0.5/rocket/serde/uuid/index.html
[`rocket::shield`]: https://api.rocket.rs/v0.5/rocket/shield/index.html
[`rocket::fs`]: https://api.rocket.rs/v0.5/rocket/fs/index.html
[`async run()`]: https://api.rocket.rs/v0.5/rocket_sync_db_pools/index.html#handlers
[`LocalRequest::json()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalRequest.html#method.json
[`LocalRequest::msgpack()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalRequest.html#method.msgpack
[`LocalResponse::json()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalResponse.html#method.json
[`LocalResponse::msgpack()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalResponse.html#method.msgpack
[hierarchical data limits]: https://api.rocket.rs/v0.5/rocket/data/struct.Limits.html#hierarchy
[default form field values]: https://rocket.rs/v0.5/guide/requests/#defaults
[`Config::ident`]: https://api.rocket.rs/rocket/struct.Config.html#structfield.ident
[`tokio`]: https://tokio.rs/
[Figment]: https://docs.rs/figment/0.10/figment/
[`TempFile`]: https://api.rocket.rs/v0.5/rocket/fs/enum.TempFile.html
[`Contextual`]: https://rocket.rs/v0.5/guide/requests/#context
[`Capped<T>`]: https://api.rocket.rs/v0.5/rocket/data/struct.Capped.html
[default catchers]: https://rocket.rs/v0.5/guide/requests/#default-catchers
[URI types]: https://api.rocket.rs/v0.5/rocket/http/uri/index.html
[`uri!`]: https://api.rocket.rs/v0.5/rocket/macro.uri.html
[`Reference`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Reference.html
[`Asterisk`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Asterisk.html
[`Redirect`]: https://api.rocket.rs/v0.5/rocket/response/struct.Redirect.html
[`UriDisplayQuery`]: https://api.rocket.rs/v0.5/rocket/derive.UriDisplayQuery.html
[`Shield`]: https://api.rocket.rs/v0.5/rocket/shield/struct.Shield.html
[Sentinels]: https://api.rocket.rs/v0.5/rocket/trait.Sentinel.html
[`local_cache!`]: https://api.rocket.rs/v0.5/rocket/request/macro.local_cache.html
[`local_cache_once!`]: https://api.rocket.rs/v0.5/rocket/request/macro.local_cache_once.html
[`CookieJar`]: https://api.rocket.rs/v0.5/rocket/http/struct.CookieJar.html
[asynchronous streams]: https://rocket.rs/v0.5/guide/responses/#async-streams
[Server-Sent Events]: https://api.rocket.rs/v0.5/rocket/response/stream/struct.EventStream.html
[`fs::relative!`]: https://api.rocket.rs/v0.5/rocket/fs/macro.relative.html
[notification]: https://api.rocket.rs/v0.5/rocket/struct.Shutdown.html
[`Rocket`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html
[`rocket::build()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.build
[`Rocket::ignite()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.ignite
[`Rocket::launch()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.launch
[`Request::rocket()`]: https://api.rocket.rs/v0.5/rocket/request/struct.Request.html#method.rocket
[Fairings]: https://rocket.rs/v0.5/guide/fairings/
[configuration system]: https://rocket.rs/v0.5/guide/configuration/
[`Poolable`]: https://api.rocket.rs/v0.5/rocket_sync_db_pools/trait.Poolable.html
[`Config`]: https://api.rocket.rs/v0.5/rocket/struct.Config.html
[`Error`]: https://api.rocket.rs/v0.5/rocket/struct.Error.html
[`LogLevel`]: https://api.rocket.rs/v0.5/rocket/config/enum.LogLevel.html
[`Rocket::register()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.register
[`NamedFile::open`]: https://api.rocket.rs/v0.5/rocket/fs/struct.NamedFile.html#method.open
[`Request::local_cache_async()`]: https://api.rocket.rs/v0.5/rocket/request/struct.Request.html#method.local_cache_async
[`FromRequest`]: https://api.rocket.rs/v0.5/rocket/request/trait.FromRequest.html
[`Fairing`]: https://api.rocket.rs/v0.5/rocket/fairing/trait.Fairing.html
[`catcher::Handler`]: https://api.rocket.rs/v0.5/rocket/catcher/trait.Handler.html
[`route::Handler`]: https://api.rocket.rs/v0.5/rocket/route/trait.Handler.html
[`FromData`]: https://api.rocket.rs/v0.5/rocket/data/trait.FromData.html
[`AsyncRead`]: https://docs.rs/tokio/1/tokio/io/trait.AsyncRead.html
[`AsyncSeek`]: https://docs.rs/tokio/1/tokio/io/trait.AsyncSeek.html
[`rocket::local::asynchronous`]: https://api.rocket.rs/v0.5/rocket/local/asynchronous/index.html
[`rocket::local::blocking`]: https://api.rocket.rs/v0.5/rocket/local/blocking/index.html
[`Outcome`]: https://api.rocket.rs/v0.5/rocket/outcome/enum.Outcome.html
[`rocket::outcome::try_outcome!`]: https://api.rocket.rs/v0.5/rocket/outcome/macro.try_outcome.html
[`Result<T, E>` implements `Responder`]: https://api.rocket.rs/v0.5/rocket/response/trait.Responder.html#provided-implementations
[`Debug`]: https://api.rocket.rs/v0.5/rocket/response/struct.Debug.html
[`std::convert::Infallible`]: https://doc.rust-lang.org/stable/std/convert/enum.Infallible.html
[`ErrorKind::Collisions`]: https://api.rocket.rs/v0.5/rocket/error/enum.ErrorKind.html#variant.Collisions
[`rocket::http::uri::fmt`]: https://api.rocket.rs/v0.5/rocket/http/uri/fmt/index.html
[`Data::open()`]: https://api.rocket.rs/v0.5/rocket/data/struct.Data.html#method.open
[`DataStream`]: https://api.rocket.rs/v0.5/rocket/data/struct.DataStream.html
[`rocket::form`]: https://api.rocket.rs/v0.5/rocket/form/index.html
[`FromFormField`]: https://api.rocket.rs/v0.5/rocket/form/trait.FromFormField.html
[`FromForm`]: https://api.rocket.rs/v0.5/rocket/form/trait.FromForm.html
[`FlashMessage`]: https://api.rocket.rs/v0.5/rocket/request/type.FlashMessage.html
[`Flash`]: https://api.rocket.rs/v0.5/rocket/response/struct.Flash.html
[`rocket::State`]: https://api.rocket.rs/v0.5/rocket/struct.State.html
[`Segments<Path>` and `Segments<Query>`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Segments.html
[`Route::map_base()`]: https://api.rocket.rs/v0.5/rocket/route/struct.Route.html#method.map_base
[`uuid` support]: https://api.rocket.rs/v0.5/rocket/serde/uuid/index.html
[`json`]: https://api.rocket.rs/v0.5/rocket/serde/json/index.html
[`msgpack`]: https://api.rocket.rs/v0.5/rocket/serde/msgpack/index.html
[`rocket::serde::json::json!`]: https://api.rocket.rs/v0.5/rocket/serde/json/macro.json.html
[`rocket::shield::Shield`]: https://api.rocket.rs/v0.5/rocket/shield/struct.Shield.html
[`rocket::fs::FileServer`]: https://api.rocket.rs/v0.5/rocket/fs/struct.FileServer.html
[`rocket_dyn_templates`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/index.html
[`rocket_sync_db_pools`]: https://api.rocket.rs/v0.5/rocket_sync_db_pools/index.html
[multitasking]: https://rocket.rs/v0.5/guide/overview/#multitasking
[`Created<R>`]: https://api.rocket.rs/v0.5/rocket/response/status/struct.Created.html
[`Created::tagged_body`]: https://api.rocket.rs/v0.5/rocket/response/status/struct.Created.html#method.tagged_body
[raw identifiers]: https://doc.rust-lang.org/1.51.0/book/appendix-01-keywords.html#raw-identifiers
[`Rocket::config()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.config
[`Rocket::figment()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.figment
[`Rocket::state()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.state
[`Rocket::catchers()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.catchers
[`LocalRequest::inner_mut()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalRequest.html#method.inner_mut
[`RawStrBuf`]: https://api.rocket.rs/v0.5/rocket/http/struct.RawStrBuf.html
[`RawStr`]: https://api.rocket.rs/v0.5/rocket/http/struct.RawStr.html
[`RawStr::percent_encode()`]: https://api.rocket.rs/v0.5/rocket/http/struct.RawStr.html#method.percent_encode
[`RawStr::percent_encode_bytes()`]: https://api.rocket.rs/v0.5/rocket/http/struct.RawStr.html#method.percent_encode_bytes
[`RawStr::strip()`]: https://api.rocket.rs/v0.5/rocket/http/struct.RawStr.html#method.strip_prefix
[`rocket::catcher`]: https://api.rocket.rs/v0.5/rocket/catcher/index.html
[`rocket::route`]: https://api.rocket.rs/v0.5/rocket/route/index.html
[`Segments::prefix_of()`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Segments.html#method.prefix_of
[`Template::try_custom()`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/struct.Template.html#method.try_custom
[`Template::custom`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/struct.Template.html#method.custom
[`FileServer::new()`]: https://api.rocket.rs/v0.5/rocket/fs/struct.FileServer.html#method.new
[`content`]: https://api.rocket.rs/v0.5/rocket/response/content/index.html
[`rocket_db_pools`]: https://api.rocket.rs/v0.5/rocket_db_pools/index.html
[mutual TLS]: https://rocket.rs/v0.5/guide/configuration/#mutual-tls
[`Certificate`]: https://api.rocket.rs/v0.5/rocket/mtls/struct.Certificate.html
[`MediaType::with_params()`]: https://api.rocket.rs/v0.5/rocket/http/struct.MediaType.html#method.with_params
[`ContentType::with_params()`]: https://api.rocket.rs/v0.5/rocket/http/struct.ContentType.html#method.with_params
[`Host`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Host.html
[`&Host`]: https://api.rocket.rs/v0.5/rocket/http/uri/struct.Host.html
[`Request::host()`]: https://api.rocket.rs/v0.5/rocket/request/struct.Request.html#method.host
[`context!`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/macro.context.html
[`MediaType`]: https://api.rocket.rs/v0.5/rocket/http/struct.MediaType.html
[`ContentType`]: https://api.rocket.rs/v0.5/rocket/http/struct.ContentType.html
[`Method`]: https://api.rocket.rs/v0.5/rocket/http/enum.Method.html
[`(ContentType, T)`]: https://api.rocket.rs/v0.5/rocket/response/content/index.html#usage
[v0.4 to v0.5 migration guide]: https://rocket.rs/v0.5/guide/upgrading/
[contrib deprecation upgrade guide]: https://rocket.rs/v0.5/guide/upgrading/#contrib-deprecation
[FAQ]: https://rocket.rs/v0.5/guide/faq/
[`Rocket::launch()`]: https://api.rocket.rs/v0.5/rocket/struct.Rocket.html#method.launch
[`ErrorKind::Shutdown`]: https://api.rocket.rs/v0.5/rocket/error/enum.ErrorKind.html#variant.Shutdown
[shutdown fairings]: https://api.rocket.rs/v0.5/rocket/fairing/trait.Fairing.html#shutdown
[`Client::terminate()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.Client.html#method.terminate
[`rocket::execute()`]: https://api.rocket.rs/v0.5/rocket/fn.execute.html
[`CookieJar::get_pending()`]: https://api.rocket.rs/v0.5/rocket/http/struct.CookieJar.html#method.get_pending
[`Metadata::render()`]: https://api.rocket.rs/v0.5/rocket_dyn_templates/struct.Metadata.html#method.render
[`pool()`]: https://api.rocket.rs/v0.5/rocket_sync_db_pools/example/struct.ExampleDb.html#method.pool
[`Request::client_ip()`]: https://api.rocket.rs/v0.5/rocket/request/struct.Request.html#method.client_ip
[`max_blocking`]: https://api.rocket.rs/v0.5/rocket/struct.Config.html#structfield.max_blocking
[`ip_header`]: https://api.rocket.rs/v0.5/rocket/struct.Config.html#structfield.ip_header
[`LocalRequest::identity()`]: https://api.rocket.rs/v0.5/rocket/local/blocking/struct.LocalRequest.html#method.identity
[`TempFile::open()`]: https://api.rocket.rs/v0.5/rocket/fs/enum.TempFile.html#method.open
[`Error::pretty_print()`]: https://api.rocket.rs/v0.5/rocket/struct.Error.html#method.pretty_print
[Request connection upgrade APIs]: https://api.rocket.rs/v0.5/rocket/struct.Response.html#upgrading
[`rocket_ws`]: https://api.rocket.rs/v0.5/rocket_ws/
[registering]: https://api.rocket.rs/v0.5/rocket/response/struct.Response.html#method.add_upgrade
[`IoHandler`]: https://api.rocket.rs/v0.5/rocket/data/trait.IoHandler.html
[`response::status`]: https://api.rocket.rs/v0.5/rocket/response/status/index.html
[Custom form errors]: https://api.rocket.rs/v0.5/rocket/form/error/enum.ErrorKind.html#variant.Custom
[`request::Outcome`]: https://api.rocket.rs/v0.5/rocket/request/type.Outcome.html#variant.Forward
[Route `Forward` outcomes]: https://api.rocket.rs/v0.5/rocket/request/type.Outcome.html#variant.Forward
[`Outcome::Error`]: https://api.rocket.rs/v0.5/rocket/outcome/enum.Outcome.html#variant.Error
[`IntoOutcome`]: https://api.rocket.rs/v0.5/rocket/outcome/trait.IntoOutcome.html
[`MediaType::JavaScript`]: https://api.rocket.rs/v0.5/rocket/http/struct.MediaType.html#associatedconstant.JavaScript
[`TempFile::open()`]: https://api.rocket.rs/v0.5/rocket/fs/enum.TempFile.html#method.open
[`Error::pretty_print()`]: https://api.rocket.rs/v0.5/rocket/struct.Error.html#method.pretty_print
[`RouteUri`]: https://api.rocket.rs/v0.5/rocket/route/struct.RouteUri.html

# Version 0.4.10 (May 21, 2021)

## Core

  * [[`3276b8`]] Removed `unsafe` in `Origin::parse_owned()`, fixing a soundness
    issue.

[`3276b8`]: https://github.com/rwf2/Rocket/commit/3276b8

# Version 0.4.9 (May 19, 2021)

## Core

  * [[`#1645`], [`f2a56f`]] Fixed `Try` `impl FromResidual<Result> for Outcome`.

[`#1645`]: https://github.com/rwf2/Rocket/issues/1645
[`f2a56f`]: https://github.com/rwf2/Rocket/commit/f2a56f

# Version 0.4.8 (May 18, 2021)

## Core

  * [[`#1548`], [`93e88b0`]] Fixed an issue that prevented compilation under
    Windows Subsystem for Linux v1.
  * Updated `Outcome` `Try` implementation to v2 in latest nightly.
  * Minimum required `rustc` is `1.54.0-nightly (2021-05-18)`.

## Internal

  * Updated `base64` dependency to `0.13`.

[`#1548`]: https://github.com/rwf2/Rocket/issues/1548
[`93e88b0`]: https://github.com/rwf2/Rocket/commit/93e88b0

# Version 0.4.7 (Feb 09, 2021)

## Core

  * [[#1534], [`2059a6`]] Fixed a low-severity, minimal impact soundness issue
    in `uri::Formatter`.

[#1534]: https://github.com/rwf2/Rocket/issues/1534
[`2059a6`]: https://github.com/rwf2/Rocket/commit/2059a6

# Version 0.4.6 (Nov 09, 2020)

## Core

  * [[`86bd7c`]] Added default and configurable read/write timeouts:
    `read_timeout` and `write_timeout`.
  * [[`c24a96`]] Added the `sse` feature, which [enables flushing] by returning
    `io::ErrorKind::WouldBlock`.

## Docs

  * Fixed broken doc links in `contrib`.
  * Fixed database library versions in `contrib` docs.

## Internal

  * Updated source code for Rust 2018.
  * UI tests now use `trybuild` instead of `compiletest-rs`.

[`86bd7c`]: https://github.com/rwf2/Rocket/commit/86bd7c
[`c24a96`]: https://github.com/rwf2/Rocket/commit/c24a96
[enables flushing]: https://api.rocket.rs/v0.4/rocket/response/struct.Stream.html#buffering-and-blocking

# Version 0.4.5 (May 30, 2020)

## Core

  * [[#1312], [`89150f`]] Fixed a low-severity, minimal impact soundness issue in
    `LocalRequest::clone()`.
  * [[#1263], [`376f74`]] Fixed a cookie serialization issue that led to
    incorrect cookie deserialization in certain cases.
  * Removed dependency on `ring` for private cookies and thus Rocket, by
    default.
  * Added [`Origin::map_path()`] for manipulating `Origin` paths.
  * Added [`handler::Outcome::from_or_forward()`].
  * Added [`Options::NormalizeDirs`] option to `StaticFiles`.
  * Improved accessibility of default error HTML.

## Docs

  * Fixed various typos.

[#1312]: https://github.com/rwf2/Rocket/issues/1312
[`89150f`]: https://github.com/rwf2/Rocket/commit/89150f
[#1263]: https://github.com/rwf2/Rocket/issues/1263
[`376f74`]: https://github.com/rwf2/Rocket/commit/376f74
[`Origin::map_path()`]: https://api.rocket.rs/v0.4/rocket/http/uri/struct.Origin.html#method.map_path
[`handler::Outcome::from_or_forward()`]: https://api.rocket.rs/v0.4/rocket/handler/type.Outcome.html#method.from_or_forward
[`Options::NormalizeDirs`]: https://api.rocket.rs/v0.4/rocket_contrib/serve/struct.Options.html#associatedconstant.NormalizeDirs

# Version 0.4.4 (Mar 09, 2020)

## Core

  * Removed use of unsupported `cfg(debug_assertions)` in `Cargo.toml`, allowing
    for builds on latest nightlies.

## Docs

  * Fixed various broken links.

# Version 0.4.3 (Feb 29, 2020)

## Core

  * Added a new [`Debug`] `500` `Responder` that `Debug`-prints its contents on
    response.
  * Specialization on `Result` was deprecated. [`Debug`] can be used in place of
    non-`Responder` errors.
  * Fixed an issue that resulted in cookies not being set on error responses.
  * Various `Debug` implementations on Rocket types now respect formatting
    options.
  * Added `Responder`s for various HTTP status codes: [`NoContent`],
    [`Unauthorized`], [`Forbidden`], and [`Conflict`].
  * `FromParam` is implemented for `NonZero` core types.

## Codegen

  * Docs for Rocket-generated macros are now hidden.
  * Generated code now works even when prelude imports like `Some`, `Ok`, and
    `Err` are shadowed.
  * Error messages referring to responder types in routes now point to the type
    correctly.

## Docs

  * All code examples in the guide are now tested and guaranteed to compile.
  * All macros are documented in the `core` crate; `rocket_codegen` makes no
    appearances.

## Infrastructure

  * CI was moved from Travis to Azure Pipelines; Windows support is tested.
  * Rocket's chat moved to [Matrix] and [Freenode].

[`Debug`]: https://api.rocket.rs/v0.4/rocket/response/struct.Debug.html
[`NoContent`]: https://api.rocket.rs/v0.4/rocket/response/status/struct.NoContent.html
[`Unauthorized`]: https://api.rocket.rs/v0.4/rocket/response/status/struct.Unauthorized.html
[`Forbidden`]: https://api.rocket.rs/v0.4/rocket/response/status/struct.Forbidden.html
[`Conflict`]: https://api.rocket.rs/v0.4/rocket/response/status/struct.Conflict.html
[Matrix]: https://chat.mozilla.org/#/room/#rocket:mozilla.org
[Freenode]: https://kiwiirc.com/client/chat.freenode.net/#rocket

# Version 0.4.2 (Jun 28, 2019)

## Core

  * Replaced use of `FnBox` with `Box<dyn FnOnce>`.
  * Removed the stable feature gates `try_from` and `transpose_result`.
  * Derive macros are reexported alongside their respective traits.
  * Minimum required `rustc` is `1.35.0-nightly (2019-04-05)`.

## Codegen

  * `JsonValue` now implements `FromIterator`.
  * `non_snake_case` errors are silenced in generated code.
  * Minimum required `rustc` is `1.33.0-nightly (2019-01-03)`.

## Contrib

  * Allow setting custom ranks on `StaticFiles` via [`StaticFiles::rank()`].
  * `MsgPack` correctly sets a MessagePack Content-Type on responses.

## Docs

  * Fixed typos across rustdocs and guide.
  * Documented library versions in contrib database documentation.

## Infrastructure

  * Updated internal dependencies to their latest versions.

[`StaticFiles::rank()`]: https://api.rocket.rs/v0.4/rocket_contrib/serve/struct.StaticFiles.html#method.rank

# Version 0.4.1 (May 11, 2019)

## Core

  * Rocket's default `Server` HTTP header no longer overrides a user-set header.
  * Fixed encoding and decoding of certain URI characters.

## Codegen

  * Compiler diagnostic information is more reliably produced.

## Contrib

  * Database pool types now implement `DerefMut`.
  * Added support for memcache connection pools.
  * Stopped depending on default features from core.

## Docs

  * Fixed many typos across the rustdocs and guide.
  * Added guide documentation on mounting more than one route at once.

## Infrastructure

  * Testing no longer requires "bootstrapping".
  * Removed deprecated `isatty` dependency in favor of `atty`.

# Version 0.4.0 (Dec 06, 2018)

## New Features

This release includes the following new features:

  * Introduced [Typed URIs].
  * Introduced [ORM agnostic database support].
  * Introduced [Request-Local State].
  * Introduced mountable static-file serving via [`StaticFiles`].
  * Introduced automatic [live template reloading].
  * Introduced custom stateful handlers via [`Handler`].
  * Introduced [transforming] data guards via [`FromData::transform()`].
  * Introduced revamped [query string handling].
  * Introduced the [`SpaceHelmet`] security and privacy headers fairing.
  * Private cookies are gated behind a `private-cookies` default feature.
  * Added [derive for `FromFormValue`].
  * Added [derive for `Responder`].
  * Added [`Template::custom()`] for customizing templating engines including
    registering filters and helpers.
  * Cookies are automatically tracked and propagated by [`Client`].
  * Private cookies can be added to local requests with
    [`LocalRequest::private_cookie()`].
  * Release builds default to the `production` environment.
  * Keep-alive can be configured via the `keep_alive` configuration parameter.
  * Allow CLI colors and emoji to be disabled with `ROCKET_CLI_COLORS=off`.
  * Route `format` accepts [shorthands] such as `json` and `html`.
  * Implemented [`Responder` for `Status`].
  * Added [`Response::cookies()`] for retrieving response cookies.
  * All logging is disabled when `log` is set to `off`.
  * Added [`Metadata`] guard for retrieving templating information.
  * The [`Uri`] type parses URIs according to RFC 7230 into one of [`Origin`],
    [`Absolute`], or [`Authority`].
  * Added [`Outcome::and_then()`], [`Outcome::failure_then()`], and
    [`Outcome::forward_then()`].
  * Implemented `Responder` for `&[u8]`.
  * Any `T: Into<Vec<Route>>` can be [`mount()`]ed.
  * [Default rankings] range from -6 to -1, differentiating on static query
    strings.
  * Added [`Request::get_query_value()`] for retrieving a query value by key.
  * Applications can launch without a working directory.
  * Added [`State::from()`] for constructing `State` values.

[`SpaceHelmet`]: https://api.rocket.rs/v0.4/rocket_contrib/helmet/index.html
[`State::from()`]: https://api.rocket.rs/v0.4/rocket/struct.State.html#method.from
[Typed URIs]: https://rocket.rs/v0.4/guide/responses/#typed-uris
[ORM agnostic database support]: https://rocket.rs/v0.4/guide/state/#databases
[`Template::custom()`]: https://api.rocket.rs/v0.4/rocket_contrib/templates/struct.Template.html#method.custom
[`LocalRequest::private_cookie()`]: https://api.rocket.rs/v0.4/rocket/local/struct.LocalRequest.html#method.private_cookie
[`LocalRequest`]: https://api.rocket.rs/v0.4/rocket/local/struct.LocalRequest.html
[shorthands]: https://api.rocket.rs/v0.4/rocket/http/struct.ContentType.html#method.parse_flexible
[derive for `FromFormValue`]: https://api.rocket.rs/v0.4/rocket_codegen/derive.FromFormValue.html
[derive for `Responder`]: https://api.rocket.rs/v0.4/rocket_codegen/derive.Responder.html
[`Response::cookies()`]: https://api.rocket.rs/v0.4/rocket/struct.Response.html#method.cookies
[`Client`]: https://api.rocket.rs/v0.4/rocket/local/struct.Client.html
[Request-Local State]: https://rocket.rs/v0.4/guide/state/#request-local-state
[`Metadata`]: https://api.rocket.rs/v0.4/rocket_contrib/templates/struct.Metadata.html
[`Uri`]: https://api.rocket.rs/v0.4/rocket/http/uri/enum.Uri.html
[`Origin`]: https://api.rocket.rs/v0.4/rocket/http/uri/struct.Origin.html
[`Absolute`]: https://api.rocket.rs/v0.4/rocket/http/uri/struct.Absolute.html
[`Authority`]: https://api.rocket.rs/v0.4/rocket/http/uri/struct.Authority.html
[`Outcome::and_then()`]: https://api.rocket.rs/v0.4/rocket/enum.Outcome.html#method.and_then
[`Outcome::forward_then()`]: https://api.rocket.rs/v0.4/rocket/enum.Outcome.html#method.forward_then
[`Outcome::failure_then()`]: https://api.rocket.rs/v0.4/rocket/enum.Outcome.html#method.failure_then
[`StaticFiles`]: https://api.rocket.rs/v0.4/rocket_contrib/serve/struct.StaticFiles.html
[live template reloading]: https://rocket.rs/v0.4/guide/responses/#live-reloading
[`Handler`]: https://api.rocket.rs/v0.4/rocket/trait.Handler.html
[`mount()`]: https://api.rocket.rs/v0.4/rocket/struct.Rocket.html#method.mount
[`FromData::transform()`]: https://api.rocket.rs/v0.4/rocket/data/trait.FromData.html#tymethod.transform
[transforming]: https://api.rocket.rs/v0.4/rocket/data/trait.FromData.html#transforming
[query string handling]: https://rocket.rs/v0.4/guide/requests/#query-strings
[Default rankings]: https://rocket.rs/v0.4/guide/requests/#default-ranking
[`Request::get_query_value()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.get_query_value
[`Responder` for `Status`]: https://rocket.rs/v0.4/guide/responses/#status

## Codegen Rewrite

The [`rocket_codegen`] crate has been entirely rewritten using to-be-stable
procedural macro APIs. We expect nightly breakages to drop dramatically, likely
to zero, as a result. The new prelude import for Rocket applications is:

```diff
- #![feature(plugin)]
- #![plugin(rocket_codegen)]
+ #![feature(proc_macro_hygiene, decl_macro)]

- extern crate rocket;
+ #[macro_use] extern crate rocket;
```

The [`rocket_codegen`] crate should **_not_** be a direct dependency. Remove it
from your `Cargo.toml`:

```diff
[dependencies]
- rocket = "0.3"
+ rocket = "0.4"
- rocket_codegen = "0.3"
```

[`rocket_codegen`]: https://api.rocket.rs/v0.4/rocket_codegen/index.html

## Breaking Changes

This release includes many breaking changes. These changes are listed below
along with a short note about how to handle the breaking change in existing
applications when applicable.

  * **Route and catcher attributes respect function privacy.**

    To mount a route or register a catcher outside of the module it is declared,
    ensure that the handler function is marked `pub` or `crate`.

  * **Query handling syntax has been completely revamped.**

    A query parameter of `<param>` is now `<param..>`. Consider whether your
    application benefits from the revamped [query string handling].

  * **The `#[error]` attribute and `errors!` macro were removed.**

    Use `#[catch]` and `catchers!` instead.

  * **`Rocket::catch()` was renamed to [`Rocket::register()`].**

    Change calls of the form `.catch(errors![..])` to
    `.register(catchers![..])`.

  * **The `#[catch]` attribute only accepts functions with 0 or 1 argument.**

    Ensure the argument to the catcher, if any, is of type `&Request`.

  * **[`json!`] returns a [`JsonValue`], no longer needs wrapping.**

    Change instances of `Json(json!(..))` to `json!` and change the
    corresponding type to `JsonValue`.

  * **All environments default to port 8000.**

    Manually configure a port of `80` for the `stage` and `production`
    environments for the previous behavior.

  * **Release builds default to the production environment.**

    Manually set the environment to `debug` with `ROCKET_ENV=debug` for the
    previous behavior.

  * **[`Form`] and [`LenientForm`] lost a lifetime parameter, `get()` method.**

    Change a type of `Form<'a, T<'a>>` to `Form<T>` or `Form<T<'a>>`. `Form<T>`
    and `LenientForm<T>` now implement `Deref<Target = T>`, allowing for calls
    to `.get()` to be removed.

  * **[`ring`] was updated to 0.13.**

    Ensure all transitive dependencies to `ring` refer to version `0.13`.

  * **`Uri` was largely replaced by [`Origin`].**

    In general, replace the type `Uri` with `Origin`. The `base` and `uri`
    fields of [`Route`] are now of type [`Origin`]. The `&Uri` guard is now
    `&Origin`. [`Request::uri()`] now returns an [`Origin`].

  * **All items in [`rocket_contrib`] are namespaced behind modules.**

    * `Json` is now `json::Json`
    * `MsgPack` is now `msgpack::MsgPack`
    * `MsgPackError` is now `msgpack::Error`
    * `Template` is now `templates::Template`
    * `UUID` is now `uuid::Uuid`
    * `Value` is replaced by `json::JsonValue`

  * **TLS certificates require the `subjectAltName` extension.**

    Ensure that your TLS certificates contain the `subjectAltName` extension
    with a value set to your domain.

  * **Route paths, mount points, and [`LocalRequest`] URIs are strictly
    checked.**

    Ensure your mount points are absolute paths with no parameters, ensure your
    route paths are absolute paths with proper parameter syntax, and ensure that
    paths passed to `LocalRequest` are valid.

  * **[`Template::show()`] takes an `&Rocket`, doesn't accept a `root`.**

    Use [`client.rocket()`] to get a reference to an instance of `Rocket` when
    testing. Use [`Template::render()`] in routes.

  * **[`Request::remote()`] returns the _actual_ remote IP, doesn't rewrite.**

    Use [`Request::real_ip()`] or [`Request::client_ip()`] to retrieve the IP
    address from the "X-Real-IP" header if it is present.

  * **[`Bind`] variant was added to [`LaunchErrorKind`].**

    Ensure matches on `LaunchErrorKind` include or ignore the `Bind` variant.

  * **Cookies are automatically tracked and propagated by [`Client`].**

    For the previous behavior, construct a `Client` with
    [`Client::untracked()`].

  * **`UUID` was renamed to [`Uuid`].**

    Use `Uuid` instead of `UUID`.

  * **`LocalRequest::cloned_dispatch()` was removed.**

    Chain calls to `.clone().dispatch()` for the previous behavior.

  * **[`Redirect`] constructors take a generic type of `T:
    TryInto<Uri<'static>>`.**

    A call to a `Redirect` constructor with a non-`'static` `&str`  of the form
    `Redirect::to(string)` should become `Redirect::to(string.to_string())`,
    heap-allocating the string before being passed to the constructor.

  * **The [`FromData`] impl for [`Form`] and [`LenientForm`] now return an error
    of type [`FormDataError`].**

    On non-I/O errors, the form string is stored in the variant as an `&'f str`.

  * **[`Missing`] variant was added to [`ConfigError`].**

    Ensure matches on `ConfigError` include or ignore the `Missing` variant.

  * **The [`FromData`] impl for [`Json`] now returns an error of type
    [`JsonError`].**

    The previous `SerdeError` is now the `.1` member of the `JsonError` `enum`.
    Match and destruct the variant for the previous behavior.

  * **[`FromData`] is now emulated by [`FromDataSimple`].**

    Change _implementations_, not uses, of `FromData` to `FromDataSimple`.
    Consider whether your implementation could benefit from [transformations].

  * **[`FormItems`] iterates over values of type [`FormItem`].**

    Map using `.map(|item| item.key_value())` for the previous behavior.

  * **[`LaunchErrorKind::Collision`] contains a vector of the colliding routes.**

    Destruct using `LaunchErrorKind::Collision(..)` to ignore the vector.

  * **[`Request::get_param()`] and [`Request::get_segments()`] are indexed by
    _segment_, not dynamic parameter.**

    Modify the `n` argument in calls to these functions appropriately.

  * **Method-based route attributes no longer accept a keyed `path` parameter.**

    Change an attribute of the form `#[get(path = "..")]` to `#[get("..")]`.

  * **[`Json`] and [`MsgPack`] data guards no longer reject requests with an
    unexpected Content-Type**

    To approximate the previous behavior, add a `format = "json"` route
    parameter when using `Json` or `format = "msgpack"` when using `MsgPack`.

  * **Implemented [`Responder` for `Status`]. Removed `Failure`,
    `status::NoContent`, and `status::Reset` responders.**

    Replace uses of `Failure(status)` with `status` directly. Replace
    `status::NoContent` with `Status::NoContent`. Replace `status::Reset` with
    `Status::ResetContent`.

  * **[`Config::root()`] returns an `Option<&Path>` instead of an `&Path`.**

    For the previous behavior, use `config.root().unwrap()`.

  * **[`Status::new()`] is no longer `const`.**

    Construct a `Status` directly.

  * **[`Config`] constructors return a `Config` instead of a `Result<Config>`.**

  * **`ConfigError::BadCWD`, `Config.config_path` were removed.**

  * **[`Json`] no longer has a default value for its type parameter.**

  * **Using `data` on a non-payload method route is a warning instead of error.**

  * **The `raw_form_string` method of [`Form`] and [`LenientForm`] was
    removed.**

  * **Various impossible `Error` associated types are now set to `!`.**

  * **All [`AdHoc`] constructors require a name as the first parameter.**

  * **The top-level `Error` type was removed.**

[`LaunchErrorKind::Collision`]: https://api.rocket.rs/v0.4/rocket/error/enum.LaunchErrorKind.html#variant.Collision
[`json!`]: https://api.rocket.rs/v0.4/rocket_contrib/macro.json.html
[`JsonValue`]: https://api.rocket.rs/v0.4/rocket_contrib/json/struct.JsonValue.html
[`Json`]: https://api.rocket.rs/v0.4/rocket_contrib/json/struct.Json.html
[`ring`]: https://crates.io/crates/ring
[`Template::show()`]: https://api.rocket.rs/v0.4/rocket_contrib/templates/struct.Template.html#method.show
[`Template::render()`]: https://api.rocket.rs/v0.4/rocket_contrib/templates/struct.Template.html#method.render
[`client.rocket()`]: https://api.rocket.rs/v0.4/rocket/local/struct.Client.html#method.rocket
[`Request::remote()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.remote
[`Request::real_ip()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.real_ip
[`Request::client_ip()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.client_ip
[`Bind`]: https://api.rocket.rs/v0.4/rocket/error/enum.LaunchErrorKind.html#variant.Bind
[`LaunchErrorKind`]: https://api.rocket.rs/v0.4/rocket/error/enum.LaunchErrorKind.html
[`Client::untracked()`]: https://api.rocket.rs/v0.4/rocket/local/struct.Client.html#method.untracked
[`Uuid`]: https://api.rocket.rs/v0.4/rocket_contrib/uuid/struct.Uuid.html
[`Route`]: https://api.rocket.rs/v0.4/rocket/struct.Route.html
[`Redirect`]: https://api.rocket.rs/v0.4/rocket/response/struct.Redirect.html
[`Request::uri()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.uri
[`FormDataError`]: https://api.rocket.rs/v0.4/rocket/request/enum.FormDataError.html
[`FromData`]: https://api.rocket.rs/v0.4/rocket/data/trait.FromData.html
[`Form`]: https://api.rocket.rs/v0.4/rocket/request/struct.Form.html
[`LenientForm`]: https://api.rocket.rs/v0.4/rocket/request/struct.LenientForm.html
[`AdHoc`]: https://api.rocket.rs/v0.4/rocket/fairing/struct.AdHoc.html
[`Missing`]: https://api.rocket.rs/v0.4/rocket/config/enum.ConfigError.html#variant.Missing
[`ConfigError`]: https://api.rocket.rs/v0.4/rocket/config/enum.ConfigError.html
[`Rocket::register()`]: https://api.rocket.rs/v0.4/rocket/struct.Rocket.html#method.register
[`JsonError`]: https://api.rocket.rs/v0.4/rocket_contrib/json/enum.JsonError.html
[transformations]: https://api.rocket.rs/v0.4/rocket/data/trait.FromData.html#transforming
[`FromDataSimple`]: https://api.rocket.rs/v0.4/rocket/data/trait.FromDataSimple.html
[`Request::get_param()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.get_param
[`Request::get_segments()`]: https://api.rocket.rs/v0.4/rocket/struct.Request.html#method.get_segments
[`FormItem`]: https://api.rocket.rs/v0.4/rocket/request/struct.FormItem.html
[`rocket_contrib`]: https://api.rocket.rs/v0.4/rocket_contrib/index.html
[`MsgPack`]: https://api.rocket.rs/v0.4/rocket_contrib/msgpack/struct.MsgPack.html
[`Status::new()`]: https://api.rocket.rs/v0.4/rocket/http/struct.Status.html#method.new
[`Config`]: https://api.rocket.rs/v0.4/rocket/struct.Config.html
[`Config::root()`]: https://api.rocket.rs/v0.4/rocket/struct.Config.html#method.root

## General Improvements

In addition to new features, Rocket saw the following improvements:

  * Log messages now refer to routes by name.
  * Collision errors on launch name the colliding routes.
  * Launch fairing failures refer to the failing fairing by name.
  * The default `403` catcher now references authorization, not authentication.
  * Private cookies are set to `HttpOnly` and are given an expiration date of 1
    week by default.
  * A [Tera templates example] was added.
  * All macros, derives, and attributes are individually documented in
    [`rocket_codegen`].
  * Invalid client requests receive a response of `400` instead of `500`.
  * Response bodies are reliably stripped on `HEAD` requests.
  * Added a default catcher for `504: Gateway Timeout`.
  * Configuration information is logged in all environments.
  * Use of `unsafe` was reduced from 9 to 2 in core library.
  * [`FormItems`] now parses empty keys and values as well as keys without
    values.
  * Added [`Config::active()`] as a shorthand for
    `Config::new(Environment::active()?)`.
  * Address/port binding errors at launch are detected and explicitly emitted.
  * [`Flash`] cookies are cleared only after they are inspected.
  * `Sync` bound on [`AdHoc::on_attach()`], [`AdHoc::on_launch()`] was removed.
  * [`AdHoc::on_attach()`], [`AdHoc::on_launch()`] accept an `FnOnce`.
  * Added [`Config::root_relative()`] for retrieving paths relative to the
    configuration file.
  * Added [`Config::tls_enabled()`] for determining whether TLS is actively
    enabled.
  * ASCII color codes are not emitted on versions of Windows that do not support
    them.
  * Added FLAC (`audio/flac`), Icon (`image/x-icon`), WEBA (`audio/webm`), TIFF
    (`image/tiff`), AAC (`audio/aac`), Calendar (`text/calendar`), MPEG
    (`video/mpeg`), TAR (`application/x-tar`), GZIP (`application/gzip`), MOV
    (`video/quicktime`), MP4 (`video/mp4`), ZIP (`application/zip`) as known
    media types.
  * Added `.weba` (`WEBA`), `.ogv` (`OGG`), `.mp4` (`MP4`), `.mpeg4` (`MP4`),
    `.aac` (`AAC`), `.ics` (`Calendar`), `.bin` (`Binary`), `.mpg` (`MPEG`),
    `.mpeg` (`MPEG`), `.tar` (`TAR`), `.gz` (`GZIP`), `.tif` (`TIFF`), `.tiff`
    (`TIFF`), `.mov` (`MOV`) as known extensions.
  * Interaction between route attributes and declarative macros has been
    improved.
  * Generated code now logs through logging infrastructures as opposed to using
    `println!`.
  * Routing has been optimized by caching routing metadata.
  * [`Form`] and [`LenientForm`] can be publicly constructed.
  * Console coloring uses default terminal colors instead of white.
  * Console coloring is consistent across all messages.
  * `i128` and `u128` now implement [`FromParam`], [`FromFormValue`].
  * The `base64` dependency was updated to `0.10`.
  * The `log` dependency was updated to `0.4`.
  * The `handlebars` dependency was updated to `1.0`.
  * The `tera` dependency was updated to `0.11`.
  * The `uuid` dependency was updated to `0.7`.
  * The `rustls` dependency was updated to `0.14`.
  * The `cookie` dependency was updated to `0.11`.

[Tera templates example]: https://github.com/rwf2/Rocket/tree/v0.4/examples/tera_templates
[`FormItems`]: https://api.rocket.rs/v0.4/rocket/request/enum.FormItems.html
[`Config::active()`]: https://api.rocket.rs/v0.4/rocket/config/struct.Config.html#method.active
[`Flash`]: https://api.rocket.rs/v0.4/rocket/response/struct.Flash.html
[`AdHoc::on_attach()`]: https://api.rocket.rs/v0.4/rocket/fairing/struct.AdHoc.html#method.on_attach
[`AdHoc::on_launch()`]: https://api.rocket.rs/v0.4/rocket/fairing/struct.AdHoc.html#method.on_launch
[`Config::root_relative()`]: https://api.rocket.rs/v0.4/rocket/struct.Config.html#method.root_relative
[`Config::tls_enabled()`]: https://api.rocket.rs/v0.4/rocket/struct.Config.html#method.tls_enabled
[`rocket_codegen`]: https://api.rocket.rs/v0.4/rocket_codegen/index.html
[`FromParam`]: https://api.rocket.rs/v0.4/rocket/request/trait.FromParam.html
[`FromFormValue`]: https://api.rocket.rs/v0.4/rocket/request/trait.FromFormValue.html
[`Data`]: https://api.rocket.rs/v0.4/rocket/struct.Data.html

## Infrastructure

  * All documentation is versioned.
  * Previous, current, and development versions of all documentation are hosted.
  * The repository was reorganized with top-level directories of `core` and
    `contrib`.
  * The `http` module was split into its own `rocket_http` crate. This is an
    internal change only.
  * All uses of `unsafe` are documented with informal proofs of correctness.

# Version 0.3.16 (Aug 24, 2018)

## Codegen

  * Codegen was updated for `2018-08-23` nightly.
  * Minimum required `rustc` is `1.30.0-nightly 2018-08-23`.

## Core

  * Force close only the read end of connections. This allows responses to be
    sent even when the client transmits more data than expected.

## Docs

  * Add details on retrieving configuration extras to guide.

# Version 0.3.15 (Jul 16, 2018)

## Codegen

  * The `#[catch]` decorator and `catchers!` macro were introduced, replacing
    `#[error]` and `errors!`.
  * The `#[error]` decorator and `errors!` macro were deprecated.
  * Codegen was updated for `2018-07-15` nightly.
  * Minimum required `rustc` is `1.29.0-nightly 2018-07-15`.

# Version 0.3.14 (Jun 22, 2018)

## Codegen

  * Codegen was updated for `2018-06-22` nightly.
  * Minimum required `rustc` is `1.28.0-nightly 2018-06-22`.

# Version 0.3.13 (Jun 16, 2018)

## Codegen

  * Codegen was updated for `2018-06-12` nightly.
  * Minimum required `rustc` is `1.28.0-nightly 2018-06-12`.

# Version 0.3.12 (May 31, 2018)

## Codegen

  * Codegen was updated for `2018-05-30` nightly.
  * Minimum required `rustc` is `1.28.0-nightly 2018-05-30`.

# Version 0.3.11 (May 19, 2018)

## Core

  * Core was updated for `2018-05-18` nightly.

## Infrastructure

  * Fixed injection of dependencies for codegen compile-fail tests.

# Version 0.3.10 (May 05, 2018)

## Core

  * Fixed parsing of nested TOML structures in config environment variables.

## Codegen

  * Codegen was updated for `2018-05-03` nightly.
  * Minimum required `rustc` is `1.27.0-nightly 2018-05-04`.

## Contrib

  * Contrib was updated for `2018-05-03` nightly.

## Docs

  * Fixed database pool type in state guide.

# Version 0.3.9 (Apr 26, 2018)

## Core

  * Core was updated for `2018-04-26` nightly.
  * Minimum required `rustc` is `1.27.0-nightly 2018-04-26`.
  * Managed state retrieval cost was reduced to an unsynchronized `HashMap`
    lookup.

## Codegen

  * Codegen was updated for `2018-04-26` nightly.
  * Minimum required `rustc` is `1.27.0-nightly 2018-04-26`.

## Contrib

  * A 512-byte buffer is preallocated when deserializing JSON, improving
    performance.

## Docs

  * Fixed various typos in rustdocs and guide.

# Version 0.3.8 (Apr 07, 2018)

## Codegen

  * Codegen was updated for `2018-04-06` nightly.
  * Minimum required `rustc` is `1.27.0-nightly 2018-04-06`.

# Version 0.3.7 (Apr 03, 2018)

## Core

  * Fixed a bug where incoming request URIs would match routes with the same
    path prefix and suffix and ignore the rest.
  * Added known media types for WASM, WEBM, OGG, and WAV.
  * Fixed fragment URI parsing.

## Codegen

  * Codegen was updated for `2018-04-03` nightly.
  * Minimum required `rustc` is `1.27.0-nightly 2018-04-03`.

## Contrib

  * JSON data is read eagerly, improving deserialization performance.

## Docs

  * Database example and docs were updated for Diesel 1.1.
  * Removed outdated README performance section.
  * Fixed various typos in rustdocs and guide.

## Infrastructure

  * Removed gates for stabilized features: `iterator_for_each`, `i128_type`,
    `conservative_impl_trait`, `never_type`.
  * Travis now tests in both debug and release mode.

# Version 0.3.6 (Jan 12, 2018)

## Core

  * `Rocket.state()` method was added to retrieve managed state from `Rocket`
    instances.
  * Nested calls to `Rocket.attach()` are now handled correctly.
  * JSON API (`application/vnd.api+json`) is now a known media type.
  * Uncached markers for `ContentType` and `Accept` headers are properly
    preserved on `Request.clone()`.
  * Minimum required `rustc` is `1.25.0-nightly 2018-01-12`.

## Codegen

  * Codegen was updated for `2017-12-22` nightly.
  * Minimum required `rustc` is `1.24.0-nightly 2017-12-22`.

## Docs

  * Fixed typo in state guide: ~~simple~~ simply.
  * Database example and docs were updated for Diesel 1.0.

## Infrastructure

  * Shell scripts now use `git grep` instead of `egrep` for faster searching.

# Version 0.3.5 (Dec 18, 2017)

## Codegen

  * Codegen was updated for `2017-12-17` nightly.
  * Minimum required `rustc` is `1.24.0-nightly 2017-12-17`.

# Version 0.3.4 (Dec 14, 2017)

## Core

  * `NamedFile`'s `Responder` implementation now uses a sized body when the
    file's length is known.
  * `#[repr(C)]` is used on `str` wrappers to guarantee correct structure layout
    across platforms.
  * A `status::BadRequest` `Responder` was added.

## Codegen

  * Codegen was updated for `2017-12-13` nightly.
  * Minimum required `rustc` is `1.24.0-nightly 2017-12-13`.

## Docs

  * The rustdoc `html_root_url` now points to the correct address.
  * Fixed typo in fairings guide: ~~event~~ events.
  * Fixed typo in `Outcome` docs: ~~users~~ Users.

# Version 0.3.3 (Sep 25, 2017)

## Core

  * `Config`'s `Debug` implementation now respects formatting options.
  * `Cow<str>` now implements `FromParam`.
  * `Vec<u8>` now implements `Responder`.
  * Added a `Binary` media type for `application/octet-stream`.
  * Empty fairing collections are no longer logged.
  * Emojis are no longer emitted to non-terminals.
  * Minimum required `rustc` is `1.22.0-nightly 2017-09-13`.

## Codegen

  * Improved "missing argument in handler" compile-time error message.
  * Codegen was updated for `2017-09-25` nightly.
  * Minimum required `rustc` is `1.22.0-nightly 2017-09-25`.

## Docs

  * Fixed typos in site overview: ~~by~~ be, ~~`Reponder`~~ `Responder`.
  * Markdown indenting was adjusted for CommonMark.

## Infrastructure

  * Shell scripts handle paths with spaces.

# Version 0.3.2 (Aug 15, 2017)

## Core

  * Added conversion methods from and to `Box<UncasedStr>`.

## Codegen

  * Lints were removed due to compiler instability. Lints will likely return as
    a separate `rocket_lints` crate.

# Version 0.3.1 (Aug 11, 2017)

## Core

  * Added support for ASCII colors on modern Windows consoles.
  * Form field renames can now include _any_ valid characters, not just idents.

## Codegen

  * Ignored named route parameters are now allowed (`_ident`).
  * Fixed issue where certain paths would cause a lint `assert!` to fail
    ([#367](https://github.com/rwf2/Rocket/issues/367)).
  * Lints were updated for `2017-08-10` nightly.
  * Minimum required `rustc` is `1.21.0-nightly (2017-08-10)`.

## Contrib

  * Tera errors that were previously skipped internally are now emitted.

## Documentation

  * Typos were fixed across the board.

# Version 0.3.0 (Jul 14, 2017)

## New Features

This release includes the following new features:

  * [Fairings], Rocket's structure middleware, were introduced.
  * [Native TLS support] was introduced.
  * [Private cookies] were introduced.
  * A [`MsgPack`] type has been added to [`contrib`] for simple consumption and
    returning of MessagePack data.
  * Launch failures ([`LaunchError`]) from [`Rocket::launch()`] are now returned
    for inspection without panicking.
  * Routes without query parameters now match requests with or without query
    parameters.
  * [Default rankings] range from -4 to -1, preferring static paths and routes
    with query string matches.
  * A native [`Accept`] header structure was added.
  * The [`Accept`] request header can be retrieved via [`Request::accept()`].
  * Incoming form fields [can be renamed] via a new `#[form(field = "name")]`
    structure field attribute.
  * All active routes can be retrieved via [`Rocket::routes()`].
  * [`Response::body_string()`] was added to retrieve the response body as a
    `String`.
  * [`Response::body_bytes()`] was added to retrieve the response body as a
    `Vec<u8>`.
  * [`Response::content_type()`] was added to easily retrieve the Content-Type
    header of a response.
  * Size limits on incoming data are [now
    configurable](https://rocket.rs/v0.3/guide/configuration/#data-limits).
  * [`Request::limits()`] was added to retrieve incoming data limits.
  * Responders may dynamically adjust their response based on the incoming
    request.
  * [`Request::guard()`] was added for simple retrieval of request guards.
  * [`Request::route()`] was added to retrieve the active route, if any.
  * `&Route` is now a request guard.
  * The base mount path of a [`Route`] can be retrieved via `Route::base` or
    `Route::base()`.
  * [`Cookies`] supports _private_ (authenticated encryption) cookies, encrypted
    with the `secret_key` config key.
  * `Config::{development, staging, production}` constructors were added for
    [`Config`].
  * [`Config::get_datetime()`] was added to retrieve an extra as a `Datetime`.
  * Forms can be now parsed _leniently_ via the new [`LenientForm`] data guard.
  * The `?` operator can now be used with `Outcome`.
  * Quoted string, array, and table  based [configuration parameters] can be set
    via environment variables.
  * Log coloring is disabled when `stdout` is not a TTY.
  * [`FromForm`] is implemented for `Option<T: FromForm>`, `Result<T: FromForm,
    T::Error>`.
  * The [`NotFound`] responder was added for simple **404** response
    construction.

[Fairings]: https://rocket.rs/v0.3/guide/fairings/
[Native TLS support]: https://rocket.rs/v0.3/guide/configuration/#configuring-tls
[Private cookies]: https://rocket.rs/v0.3/guide/requests/#private-cookies
[can be renamed]: https://rocket.rs/v0.3/guide/requests/#field-renaming
[`MsgPack`]: https://api.rocket.rs/v0.3/rocket_contrib/struct.MsgPack.html
[`Rocket::launch()`]: https://api.rocket.rs/v0.3/rocket/struct.Rocket.html#method.launch
[`LaunchError`]: https://api.rocket.rs/v0.3/rocket/error/struct.LaunchError.html
[Default rankings]: https://api.rocket.rs/v0.3/rocket/struct.Route.html
[`Route`]: https://api.rocket.rs/v0.3/rocket/struct.Route.html
[`Accept`]: https://api.rocket.rs/v0.3/rocket/http/struct.Accept.html
[`Request::accept()`]: https://api.rocket.rs/v0.3/rocket/struct.Request.html#method.accept
[`contrib`]: https://api.rocket.rs/v0.3/rocket_contrib/
[`Rocket::routes()`]: https://api.rocket.rs/v0.3/rocket/struct.Rocket.html#method.routes
[`Response::body_string()`]: https://api.rocket.rs/v0.3/rocket/struct.Response.html#method.body_string
[`Response::body_bytes()`]: https://api.rocket.rs/v0.3/rocket/struct.Response.html#method.body_bytes
[`Response::content_type()`]: https://api.rocket.rs/v0.3/rocket/struct.Response.html#method.content_type
[`Request::guard()`]: https://api.rocket.rs/v0.3/rocket/struct.Request.html#method.guard
[`Request::limits()`]: https://api.rocket.rs/v0.3/rocket/struct.Request.html#method.limits
[`Request::route()`]: https://api.rocket.rs/v0.3/rocket/struct.Request.html#method.route
[`Config`]: https://api.rocket.rs/v0.3/rocket/struct.Config.html
[`Cookies`]: https://api.rocket.rs/v0.3/rocket/http/enum.Cookies.html
[`Config::get_datetime()`]: https://api.rocket.rs/v0.3/rocket/struct.Config.html#method.get_datetime
[`LenientForm`]: https://api.rocket.rs/v0.3/rocket/request/struct.LenientForm.html
[configuration parameters]: https://api.rocket.rs/v0.3/rocket/config/index.html#environment-variables
[`NotFound`]: https://api.rocket.rs/v0.3/rocket/response/status/struct.NotFound.html

## Breaking Changes

This release includes many breaking changes. These changes are listed below
along with a short note about how to handle the breaking change in existing
applications.

  * **`session_key` was renamed to `secret_key`, requires a 256-bit base64 key**

    It's unlikely that `session_key` was previously used. If it was, rename
    `session_key` to `secret_key`. Generate a random 256-bit base64 key using a
    tool like openssl: `openssl rand -base64 32`.

  * **The `&Cookies` request guard has been removed in favor of `Cookies`**

    Change `&Cookies` in a request guard position to `Cookies`.

  * **`Rocket::launch()` now returns a `LaunchError`, doesn't panic.**

    For the old behavior, suffix a call to `.launch()` with a semicolon:
    `.launch();`.

  * **Routes without query parameters match requests with or without query
    parameters.**

    There is no workaround, but this change may allow manual ranks from routes
    to be removed.

  * **The `format` route attribute on non-payload requests matches against the
    Accept header.**

    Excepting a custom request guard, there is no workaround. Previously,
    `format` always matched against the Content-Type header, regardless of
    whether the request method indicated a payload or not.

  * **A type of `&str` can no longer be used in form structures or parameters.**

    Use the new [`&RawStr`] type instead.

  * **`ContentType` is no longer a request guard.**

    Use `&ContentType` instead.

  * **`Request::content_type()` returns `&ContentType` instead of
    `ContentType`.**

    Use `.clone()` on `&ContentType` if a type of `ContentType` is required.

  * **`Response::header_values()` was removed. `Response::headers()` now returns
    an `&HeaderMap`.**

    A call to `Response::headers()` can be replaced with
    `Response::headers().iter()`. A call to `Response::header_values(name)` can
    be replaced with `Response::headers().get(name)`.

  * **Route collisions result in a hard error and panic.**

    There is no workaround. Previously, route collisions were a warning.

  * **The [`IntoOutcome`] trait has been expanded and made more flexible.**

    There is no workaround. `IntoOutcome::into_outcome()` now takes a `Failure`
    value to use. `IntoOutcome::or_forward()` was added to return a `Forward`
    outcome if `self` indicates an error.

  * **The 'testing' feature was removed.**

    Remove `features = ["testing"]` from `Cargo.toml`. Use the new [`local`]
    module for testing.

  * **`serde` was updated to 1.0.**

    There is no workaround. Ensure all dependencies rely on `serde` `1.0`.

  * **`config::active()` was removed.**

    Use [`Rocket::config()`] to retrieve the configuration before launch. If
    needed, use [managed state] to store config information for later use.

  * **The [`Responder`] trait has changed.**

    `Responder::respond(self)` was removed in favor of
    `Responder::respond_to(self, &Request)`. Responders may dynamically adjust
    their response based on the incoming request.

  * **`Outcome::of(Responder)` was removed while `Outcome::from(&Request,
    Responder)` was added.**

    Use `Outcome::from(..)` instead of `Outcome::of(..)`.

  * **Usage of templates requires `Template::fairing()` to be attached.**

    Call `.attach(Template::fairing())` on the application's Rocket instance
    before launching.

  * **The `Display` implementation of `Template` was removed.**

    Use [`Template::show()`] to render a template directly.

  * **`Request::new()` is no longer exported.**

    There is no workaround.

  * **The [`FromForm`] trait has changed.**

    `Responder::from_form_items(&mut FormItems)` was removed in favor of
    `Responder::from_form(&mut FormItems, bool)`. The second parameter indicates
    whether parsing should be strict (if `true`) or lenient (if `false`).

  * **`LoggingLevel` was removed as a root reexport.**

    It can now be imported from `rocket::config::LoggingLevel`.

  * **An `Io` variant was added to [`ConfigError`].**

    Ensure `match`es on `ConfigError` include an `Io` variant.

  * **[`ContentType::from_extension()`] returns an `Option<ContentType>`.**

    For the old behavior, use `.unwrap_or(ContentType::Any)`.

  * **The `IntoValue` config trait was removed in favor of `Into<Value>`.**

    There is no workaround. Use `Into<Value>` as necessary.

  * **The `rocket_contrib::JSON` type has been renamed to
    [`rocket_contrib::Json`].**

    Use `Json` instead of `JSON`.

  * **All structs in the [`content`] module use TitleCase names.**

    Use `Json`, `Xml`, `Html`, and `Css` instead of `JSON`, `XML`, `HTML`, and
    `CSS`, respectively.

[`&RawStr`]: https://api.rocket.rs/v0.3/rocket/http/struct.RawStr.html
[`IntoOutcome`]: https://api.rocket.rs/v0.3/rocket/outcome/trait.IntoOutcome.html
[`local`]: https://api.rocket.rs/v0.3/rocket/local/index.html
[`Rocket::config()`]: https://api.rocket.rs/v0.3/rocket/struct.Rocket.html#method.config
[managed state]: https://rocket.rs/v0.3/guide/state/
[`Responder`]: https://api.rocket.rs/v0.3/rocket/response/trait.Responder.html
[`Template::show()`]: https://api.rocket.rs/v0.3/rocket_contrib/struct.Template.html#method.show
[`FromForm`]: https://api.rocket.rs/v0.3/rocket/request/trait.FromForm.html
[`ConfigError`]: https://api.rocket.rs/v0.3/rocket/config/enum.ConfigError.html
[`ContentType::from_extension()`]: https://api.rocket.rs/v0.3/rocket/http/struct.ContentType.html#method.from_extension
[`rocket_contrib::Json`]: https://api.rocket.rs/v0.3/rocket_contrib/struct.Json.html
[`content`]: https://api.rocket.rs/v0.3/rocket/response/content/index.html

## General Improvements

In addition to new features, Rocket saw the following improvements:

  * "Rocket" is now capitalized in the `Server` HTTP header.
  * The generic parameter of `rocket_contrib::Json` defaults to `json::Value`.
  * The trailing '...' in the launch message was removed.
  * The launch message prints regardless of the config environment.
  * For debugging, `FromData` is implemented for `Vec<u8>` and `String`.
  * The port displayed on launch is the port resolved, not the one configured.
  * The `uuid` dependency was updated to `0.5`.
  * The `base64` dependency was updated to `0.6`.
  * The `toml` dependency was updated to `0.4`.
  * The `handlebars` dependency was updated to `0.27`.
  * The `tera` dependency was updated to `0.10`.
  * [`yansi`] is now used for all terminal coloring.
  * The `dev` `rustc` release channel is supported during builds.
  * [`Config`] is now exported from the root.
  * [`Request`] implements `Clone` and `Debug`.
  * The `workers` config parameter now defaults to `num_cpus * 2`.
  * Console logging for table-based config values is improved.
  * `PartialOrd`, `Ord`, and `Hash` are now implemented for [`State`].
  * The format of a request is always logged when available.
  * Route matching on `format` now functions as documented.

[`yansi`]: https://crates.io/crates/yansi
[`Request`]: https://api.rocket.rs/v0.3/rocket/struct.Request.html
[`State`]: https://api.rocket.rs/v0.3/rocket/struct.State.html

## Infrastructure

  * All examples include a test suite.
  * The `master` branch now uses a `-dev` version number.

# Version 0.2.8 (Jun 01, 2017)

## Codegen

  * Lints were updated for `2017-06-01` nightly.
  * Minimum required `rustc` is `1.19.0-nightly (2017-06-01)`.

# Version 0.2.7 (May 26, 2017)

## Codegen

  * Codegen was updated for `2017-05-26` nightly.

# Version 0.2.6 (Apr 17, 2017)

## Codegen

  * Allow `k` and `v` to be used as fields in `FromForm` structures by avoiding
    identifier collisions ([#265]).

[#265]: https://github.com/rwf2/Rocket/issues/265

# Version 0.2.5 (Apr 16, 2017)

## Codegen

  * Lints were updated for `2017-04-15` nightly.
  * Minimum required `rustc` is `1.18.0-nightly (2017-04-15)`.

# Version 0.2.4 (Mar 30, 2017)

## Codegen

  * Codegen was updated for `2017-03-30` nightly.
  * Minimum required `rustc` is `1.18.0-nightly (2017-03-30)`.

# Version 0.2.3 (Mar 22, 2017)

## Fixes

  * Multiple header values for the same header name are now properly preserved
    (#223).

## Core

  * The `get_slice` and `get_table` methods were added to `Config`.
  * The `pub_restricted` feature has been stabilized!

## Codegen

  * Lints were updated for `2017-03-20` nightly.
  * Minimum required `rustc` is `1.17.0-nightly (2017-03-22)`.

## Infrastructure

  * The test script now denies trailing whitespace.

# Version 0.2.2 (Feb 26, 2017)

## Codegen

  * Lints were updated for `2017-02-25`  and `2017-02-26` nightlies.
  * Minimum required `rustc` is `1.17.0-nightly (2017-02-26)`.

# Version 0.2.1 (Feb 24, 2017)

## Core Fixes

  * `Flash` cookie deletion functions as expected regardless of the path.
  * `config` properly accepts IPv6 addresses.
  * Multiple `Set-Cookie` headers are properly set.

## Core Improvements

  * `Display` and `Error` were implemented for `ConfigError`.
  * `webp`, `ttf`, `otf`, `woff`, and `woff2` were added as known content types.
  * Routes are presorted for faster routing.
  * `into_bytes` and `into_inner` methods were added to `Body`.

## Codegen

  * Fixed `unmanaged_state` lint so that it works with prefilled type aliases.

## Contrib

  * Better errors are emitted on Tera template parse errors.

## Documentation

  * Fixed typos in `manage` and `JSON` docs.

## Infrastructure

  * Updated doctests for latest Cargo nightly.

# Version 0.2.0 (Feb 06, 2017)

Detailed release notes for v0.2 can also be found on
[rocket.rs](https://rocket.rs/v0.3/news/2017-02-06-version-0.2/).

## New Features

This release includes the following new features:

  * Introduced managed state.
  * Added lints that warn on unmanaged state and unmounted routes.
  * Added the ability to set configuration parameters via environment variables.
  * `Config` structures can be built via `ConfigBuilder`, which follows the
    builder pattern.
  * Logging can be enabled or disabled on custom configuration via a second
    parameter to the `Rocket::custom` method.
  * `name` and `value` methods were added to `Header` to retrieve the name and
    value of a header.
  * A new configuration parameter, `workers`, can be used to set the number of
    threads Rocket uses.
  * The address of the remote connection is available via `Request.remote()`.
    Request preprocessing overrides remote IP with value from the `X-Real-IP`
    header, if present.
  * During testing, the remote address can be set via `MockRequest.remote()`.
  * The `SocketAddr` request guard retrieves the remote address.
  * A `UUID` type has been added to `contrib`.
  * `rocket` and `rocket_codegen` will refuse to build with an incompatible
    nightly version and emit nice error messages.
  * Major performance and usability improvements were upstreamed to the `cookie`
    crate, including the addition of a `CookieBuilder`.
  * When a checkbox isn't present in a form, `bool` types in a `FromForm`
    structure will parse as `false`.
  * The `FormItems` iterator can be queried for a complete parse via `completed`
    and `exhausted`.
  * Routes for `OPTIONS` requests can be declared via the `options` decorator.
  * Strings can be percent-encoded via `URI::percent_encode()`.

## Breaking Changes

This release includes several breaking changes. These changes are listed below
along with a short note about how to handle the breaking change in existing
applications.

  * **`Rocket::custom` takes two parameters, the first being `Config` by
    value.**

    A call in v0.1 of the form `Rocket::custom(&config)` is now
    `Rocket::custom(config, false)`.

  * **Tera templates are named without their extension.**

    A templated named `name.html.tera` is now simply `name`.

  * **`JSON` `unwrap` method has been renamed to `into_inner`.**

    A call to `.unwrap()` should be changed to `.into_inner()`.

  * **The `map!` macro was removed in favor of the `json!` macro.**

    A call of the form `map!{ "a" => b }` can be written as: `json!({ "a": b
    })`.

  * **The `hyper::SetCookie` header is no longer exported.**

    Use the `Cookie` type as an `Into<Header>` type directly.

  * **The `Content-Type` for `String` is now `text/plain`.**

    Use `content::HTML<String>` for HTML-based `String` responses.

  * **`Request.content_type()` returns an `Option<ContentType>`.**

    Use `.unwrap_or(ContentType::Any)` to get the old behavior.

  * **The `ContentType` request guard forwards when the request has no
    `Content-Type` header.**

    Use an `Option<ContentType>` and `.unwrap_or(ContentType::Any)` for the old
    behavior.

  * **A `Rocket` instance must be declared _before_ a `MockRequest`.**

    Change the order of the `rocket::ignite()` and `MockRequest::new()` calls.

  * **A route with `format` specified only matches requests with the same
    format.**

    Previously, a route with a `format` would match requests without a format
    specified. There is no workaround to this change; simply specify formats
    when required.

  * **`FormItems` can no longer be constructed directly.**

    Instead of constructing as `FormItems(string)`, construct as
    `FormItems::from(string)`.

  * **`from_from_string(&str)` in `FromForm` removed in favor of
    `from_form_items(&mut FormItems)`.**

    Most implementation should be using `FormItems` internally; simply use the
    passed in `FormItems`. In other cases, the form string can be retrieved via
    the `inner_str` method of `FormItems`.

  * **`Config::{set, default_for}` are deprecated.**

    Use the `set_{param}` methods instead of `set`, and `new` or `build` in
    place of `default_for`.

  * **Route paths must be absolute.**

    Prepend a `/` to convert a relative path into an absolute one.

  * **Route paths cannot contain empty segments.**

    Remove any empty segments, including trailing ones, from a route path.

## Bug Fixes

A couple of bugs were fixed in this release:

  * Handlebars partials were not properly registered
    ([#122](https://github.com/rwf2/Rocket/issues/122)).
  * `Rocket::custom` did not set the custom configuration as the `active`
    configuration.
  * Route path segments containing more than one dynamic parameter were
    allowed.

## General Improvements

In addition to new features, Rocket saw the following smaller improvements:

  * Rocket no longer overwrites a catcher's response status.
  * The `port` `Config` type is now a proper `u16`.
  * Clippy issues injected by codegen are resolved.
  * Handlebars was updated to `0.25`.
  * The `PartialEq` implementation of `Config` doesn't consider the path or
    secret key.
  * Hyper dependency updated to `0.10`.
  * The `Error` type for `JSON as FromData` has been exposed as `SerdeError`.
  * SVG was added as a known Content-Type.
  * Serde was updated to `0.9`.
  * Form parse failure now results in a **422** error code.
  * Tera has been updated to `0.7`.
  * `pub(crate)` is used throughout to enforce visibility rules.
  * Query parameters in routes (`/path?<param>`) are now logged.
  * Routes with and without query parameters no longer _collide_.

## Infrastructure

  * Testing was parallelized, resulting in 3x faster Travis builds.

# Version 0.1.6 (Jan 26, 2017)

## Infrastructure

  * Hyper version pinned to 0.9.14 due to upstream non-semver breaking change.

# Version 0.1.5 (Jan 14, 2017)

## Core

  * Fixed security checks in `FromSegments` implementation for `PathBuf`.

## Infrastructure

  * `proc_macro` feature removed from examples due to stability.

# Version 0.1.4 (Jan 4, 2017)

## Core

  * Header names are treated as case-preserving.

## Codegen

  * Minimum supported nightly is `2017-01-03`.

# Version 0.1.3 (Dec 31, 2016)

## Core

  * Typo in `Outcome` formatting fixed (Succcess -> Success).
  * Added `ContentType::CSV`.
  * Dynamic segments parameters are properly resolved, even when mounted.
  * Request methods are only overridden via `_method` field on POST.
  * Form value `String`s are properly decoded.

## Codegen

  * The `_method` field is now properly ignored in `FromForm` derivation.
  * Unknown Content-Types in `format` no longer result in an error.
  * Deriving `FromForm` no longer results in a deprecation warning.
  * Codegen will refuse to build with incompatible rustc, presenting error
    message and suggestion.
  * Added `head` as a valid decorator for `HEAD` requests.
  * Added `route(OPTIONS)` as a valid decorator for `OPTIONS` requests.

## Contrib

  * Templates with the `.tera` extension are properly autoescaped.
  * Nested template names are properly resolved on Windows.
  * Template implements `Display`.
  * Tera dependency updated to version 0.6.

## Docs

  * Todo example requirements clarified in its `README`.

## Testing

  * Tests added for `config`, `optional_result`, `optional_redirect`, and
    `query_params` examples.
  * Testing script checks for and disallows tab characters.

## Infrastructure

  * New script (`bump_version.sh`) automates version bumps.
  * Config script emits error when readlink/readpath support is bad.
  * Travis badge points to public builds.

# Version 0.1.2 (Dec 24, 2016)

## Codegen

  * Fix `get_raw_segments` index argument in route codegen
    ([#41](https://github.com/rwf2/Rocket/issues/41)).
  * Segments params (`<param..>`) respect prefixes.

## Contrib

  * Fix nested template name resolution
    ([#42](https://github.com/rwf2/Rocket/issues/42)).

## Infrastructure

  * New script (`publish.sh`) automates publishing to crates.io.
  * New script (`bump_version.sh`) automates version bumps.

# Version 0.1.1 (Dec 23, 2016)

## Core

  * `NamedFile` `Responder` lost its body in the shuffle; it's back!

# Version 0.1.0 (Dec 23, 2016)

This is the first public release of Rocket!

## Breaking

All of the mentions to `hyper` types in core Rocket types are no more. Rocket
now implements its own `Request` and `Response` types.

  * `ContentType` uses associated constants instead of static methods.
  * `StatusCode` removed in favor of new `Status` type.
  * `Response` type alias superseded by `Response` type.
  * `Responder::respond` no longer takes in hyper type.
  * `Responder::respond` returns `Response`, takes `self` by move.
  * `Handler` returns `Outcome` instead of `Response` type alias.
  * `ErrorHandler` returns `Result`.
  * All `Hyper*` types were moved to unprefixed versions in `hyper::`.
  * `MockRequest::dispatch` now returns a `Response` type.
  * `URIBuf` removed in favor of unified `URI`.
  * Rocket panics when an illegal, dynamic mount point is used.

## Core

  * Rocket handles `HEAD` requests automatically.
  * New `Response` and `ResponseBuilder` types.
  * New `Request`, `Header`, `Status`, and `ContentType` types.

## Testing

  * `MockRequest` allows any type of header.
  * `MockRequest` allows cookies.

## Codegen

  * Debug output disabled by default.
  * The `ROCKET_CODEGEN_DEBUG` environment variables enables codegen logging.

# Version 0.0.11 (Dec 11, 2016)

## Streaming Requests

All incoming request data is now streamed. This resulted in a major change to
the Rocket APIs. They are summarized through the following API changes:

  * The `form` route parameter has been removed.
  * The `data` route parameter has been introduced.
  * Forms are now handled via the `data` parameter and `Form` type.
  * Removed the `data` parameter from `Request`.
  * Added `FromData` conversion trait and default implementation.
  * `FromData` is used to automatically derive the `data` parameter.
  * `Responder`s are now final: they cannot forward to other requests.
  * `Responder`s may only forward to catchers.

## Breaking

  * Request `uri` parameter is private. Use `uri()` method instead.
  * `form` module moved under `request` module.
  * `response::data` was renamed to `response::content`.
  * Introduced `Outcome` with `Success`, `Failure`, and `Forward` variants.
  * `outcome` module moved to top-level.
  * `Response` is now a type alias to `Outcome`.
  * `Empty` `Responder` was removed.
  * `StatusResponder` removed in favor of `response::status` module.

## Codegen

  * Error handlers can now take 0, 1, or 2 parameters.
  * `FromForm` derive now works on empty structs.
  * Lifetimes are now properly stripped in code generation.
  * Any valid ident is now allowed in single-parameter route parameters.

## Core

  * Route is now cloneable.
  * `Request` no longer has any lifetime parameters.
  * `Handler` type now includes a `Data` parameter.
  * `http` module is public.
  * `Responder` implemented for `()` type as an empty response.
  * Add `config::get()` for global config access.
  * Introduced `testing` module.
  * `Rocket.toml` allows global configuration via `[global]` table.

## Docs

  * Added a `raw_upload` example.
  * Added a `pastebin` example.
  * Documented all public APIs.

## Testing

  * Now building and running tests with `--all-features` flag.
  * Added appveyor config for Windows CI testing.

# Version 0.0.10 (Oct 03, 2016)

## Breaking

  * Remove `Rocket::new` in favor of `ignite` method.
  * Remove `Rocket::mount_and_launch` in favor of chaining `mount(..).launch()`.
  * `mount` and `catch` take `Rocket` type by value.
  * All types related to HTTP have been moved into `http` module.
  * `Template::render` in `contrib` now takes context by reference.

## Core

  * Rocket now parses option `Rocket.toml` for configuration, defaulting to sane
    values.
  * `ROCKET_ENV` environment variable can be used to specify running environment.

## Docs

  * Document `ContentType`.
  * Document `Request`.
  * Add script that builds docs.

## Testing

  * Scripts can now be run from any directory.
  * Cache Cargo directories in Travis for faster testing.
  * Check that library version numbers match in testing script.

# Version 0.0.9 (Sep 29, 2016)

## Breaking

  * Rename `response::data_type` to `response::data`.

## Core

  * Rocket interprets `_method` field in forms as the incoming request's method.
  * Add `Outcome::Bad` to signify responses that failed internally.
  * Add a `NamedFile` `Responder` type that uses a file's extension for the
    response's content type.
  * Add a `Stream` `Responder` for streaming responses.

## Contrib

  * Introduce the `contrib` crate.
  * Add JSON support via `JSON`, which implements `FromRequest` and `Responder`.
  * Add templating support via `Template` which implements `Responder`.

## Docs

  * Initial guide-like documentation.
  * Add documentation, testing, and contributing sections to README.

## Testing

  * Add a significant number of codegen tests.
