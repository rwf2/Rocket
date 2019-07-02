# Version 0.4.2 (Jun 28, 2019)

## Core

  * Replaced use of `FnBox` with `Box<dyn FnOnce>`.
  * Removed the stablized feature gates `try_from` and `transpose_result`.
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

[Tera templates example]: https://github.com/SergioBenitez/Rocket/tree/v0.4/examples/tera_templates
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
    ([#367](https://github.com/SergioBenitez/Rocket/issues/367)).
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
  * [`Cookies`] supports _private_ (authenticated encryption) cookies, encryped
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

    For the old behvavior, use `.unwrap_or(ContentType::Any)`.

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

  * "Rocket" is now capatilized in the `Server` HTTP header.
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

[#265]: https://github.com/SergioBenitez/Rocket/issues/265

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
    ([#122](https://github.com/SergioBenitez/Rocket/issues/122)).
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
    ([#41](https://github.com/SergioBenitez/Rocket/issues/41)).
  * Segments params (`<param..>`) respect prefixes.

## Contrib

  * Fix nested template name resolution
    ([#42](https://github.com/SergioBenitez/Rocket/issues/42)).

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
  * `Response` type alias superceded by `Response` type.
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
  * `Responser`s may only forward to catchers.

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

