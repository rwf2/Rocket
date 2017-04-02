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
[rocket.rs](https://rocket.rs/news/2017-02-06-version-0.2/).

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
    session key.
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

