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

