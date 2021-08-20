# Upgrading

Rocket v0.5 bring many new features and improvements over Rocket v0.4. Along
with new features, Rocket v0.5 also includes many changes that improve the
overall usability, stability, and security of the framework and applications
written in it. While the Rust compiler can guide you through many of these
changes, others require special attention. The intent of this guide is to guide
you through these changes and more, migrating your Rocket application to 0.5 and
reaping the benefits of new features and improvements.

This guide is _not_ intended to replace, but instead complement, a reading of
the [CHANGELOG]. The [CHANGELOG] should be considered required reading for all
developers wishing to migrate their applications to Rocket v0.5.

[CHANGELOG]: @github/CHANGELOG.md

## Crate Organization

Rocket v0.5 incorporates an improved module structure and crate ecosystem.
Modules and items that have been moved or removed will trigger a compiler error.
We encourage users to search through the [CHANGELOG] or [API docs](@api/rocket)
for the v0.5 analog. All previously existing functionality, except for that
incompatible with async I/O, is available in v0.5.

### Off-by-Default Secrets

The `private-cookies` crate feature, which was previously enabled by default,
has been renamed to `secrets` and is disabled by default. If you are using
private cookies, you _must_ enable the `secrets` feature in `Cargo.toml`:

```toml
[dependencies]
rocket = { version = "0.5.0-rc.1", features = ["secrets"] }
```

### Contrib Deprecation

The `rocket_contrib` crate is deprecated and is wholly incompatible with Rocket
0.5. _All_ users of `rocket_contrib` _must_:

  * Remove all dependencies and references to `rocket_contrib`.
  * For templating support, depend on the new [`rocket_dyn_templates`] crate.
  * For database pools, depend on the new [`rocket_sync_db_pools`] and/or
    [`rocket_db_pools`] crates.
  * Enable [features in `rocket`] as necessary.

For example, to make use of JSON and Tera templates, make the following changes
to `Cargo.toml`:

```diff
[dependencies]
- rocket = "0.4"
- rocket_contrib = { version = "0.4", features = ["json"], default-features = false }
+ rocket = { version = "0.5.0-rc.1", features = ["json"] }
+ rocket_dyn_templates = { version = "0.1.0-rc.1", features = ["tera"] }
```

! note: `rocket_dyn_templates` _does not_ follow in version lock-step with
  the `rocket` crate.

  This is intentional. The crate depends on many external dependencies which may
  evolve at a different pace than Rocket itself. Allowing their versions to
  diverge enables keeping dependencies up-to-date without breaking `rocket`
  itself.

All features previously present in `rocket_contrib` are available elsewhere. The
[contrib graduation] section of the CHANGELOG contains full details on the
changes.


[`rocket_dyn_templates`]: @api/rocket_dyn_templates
[`rocket_sync_db_pools`]: @api/rocket_sync_db_pools
[`rocket_db_pools`]: @api/rocket_db_pools
[features in `rocket`]: @api/rocket/#features
[contrib graduation]: @github/CHANGELOG.md#contrib-graduation

## Stable and Async Support

Rocket v0.5 compiles and builds on Rust stable with an entirely asynchronous
core. You are encouraged to:

  * Switch to the Rust stable release channel for production builds.
  * Remove the previously required `#![feature(..)]` crate attribute.

All application authors _must_:

  * Use `rocket::build()` instead of `rocket::ignite()`.
  * Use either the `#[launch]` or `#[rocket::main]` async entry attribute.
  * Use `async` versions of any blocking I/O or execute it in another thread.

Application authors _may_:

  * Prefer to explicitly import macros via `use` instead of `#[macro_use]`.

The rest of the section describes making these changes in detail.

### Stable Release Channel

If you prefer to use Rust's stable release channel, you can switch to it using
`rustup`:

```sh
## switch globally
rustup default nightly

## switch locally
rustup override set stable
```

Using the stable release channel ensures that _no_ breakages will occur when
upgrading your Rust compiler or Rocket. That being said, Rocket continues to
take advantage of features only present in the nightly channel. As a result, the
development experience will continue to be better on nightly for the forseeable
future. For example, compiler diagnostics on `nightly` are more detailed and
accurate:

<details>
<summary>Example Diagnostic on Nightly</summary>

```rust,ignore
error: invalid parameters for `has_two` route uri
  --> $DIR/typed-uris-bad-params.rs:55:18
   |
55 |     uri!(has_two(id = 100, cookies = "hi"));
   |                  ^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: uri parameters are: id: i32, name: String
   = help: missing parameter: `name`
help: unknown parameter: `cookies`
  --> $DIR/typed-uris-bad-params.rs:55:28
   |
55 |     uri!(has_two(id = 100, cookies = "hi"));
   |                            ^^^^^^^
```

</details>

<details>
<summary>Example Diagnostic on Stable</summary>

```rust,ignore
error: invalid parameters for `has_two` route uri
  --- note: uri parameters are: id: i32, name: String
  --- help: missing parameter: `name`
  --> $DIR/typed-uris-bad-params.rs:55:18
   |
55 |     uri!(has_two(id = 100, cookies = "hi"));
   |                  ^^

error: [help] unknown parameter: `cookies`
  --> $DIR/typed-uris-bad-params.rs:55:28
   |
55 |     uri!(has_two(id = 100, cookies = "hi"));
   |                            ^^^^^^^
```

</details>

Our **recommendation** is to develop locally on the nightly channel but build
and deploy for production on the stable channel.

### Feature Attribute

As a result support for the stable release channel, Rocket applications no
longer need to enable any features to be used. You should **remove any
`#[feature(..)]` crate attributes:**

```diff
- #![feature(proc_macro_hygiene, decl_macro)]
-
  #[macro_use] extern crate rocket;

  fn main() { .. }
```

### Updates to Launch

The new asynchronous core requires an async runtime to run. The new
[`launch`] and [`main`] attributes simplify starting a runtime suitable for
running Rocket applications. You should use [`launch`] whenever possible.

At the same time, the `rocket::ignite()` function has been renamed to
[`rocket::build()`]; calls to the function or method should be replaced
accordingly. Together, these two changes result in the following diff to what
was previously the `main` function:

```diff
- fn main() {
-     rocket::ignite().mount("/hello", routes![hello]).launch();
- }
+ #[launch]
+ fn rocket() -> _ {
+     rocket::build().mount("/hello", routes![hello])
+ }
```

[`launch`]: @api/rocket/attr.launch.html
[`main`]: @api/rocket/attr.main.html
[`rocket::build()`]: @api/rocket/struct.Rocket.html#method.build

### Blocking I/O

Rocket v0.5 takes advantage of the latest developments in async I/O in Rust by
migrating to a fully asynchronous core powered by [`tokio`]. Specifically,
_every_ request is handled by an asynchronous task which internally calls one or
more request handlers. Asynchronous tasks are multiplexed on a [configurable
number of worker threads]. Though there is no limit on the number of tasks that
can run concurrently, at most `worker` tasks can be run in parallel.

The runtime can switch between tasks in a single worker thread _iff_ an `await`
point in reached. In other words, context switching is _cooperative_, _not_
preemptive. This _iff_ is critical: if an `await` point is _not_ reached, no
task switching can occur. As such, it is important that `await` points occur
periodically in a task so that tasks waiting to be scheduled are not starved.

In general, when working with `async` APIs, await points will occur naturally.
However, an application written for synchronous I/O, like all Rocket
applications prior to 0.4, must take great care to convert all synchronous,
blocking I/O, to `async` I/O. This is because, as the name implies, blocking I/O
blocks a thread from making progress until the I/O result is available, meaning
that no tasks can be scheduled on the waiting thread, wasting valuable resources
and significantly degrading performance.

Common sources of blocking I/O and their `async` replacements include:

  * Anything in `std::fs`: replace with `rocket::tokio::fs`.
  * Anything in `std::sync`: replace with `rocket::tokio::sync`.
  * Anything in `std::net`: replace with `rocket::tokio::net`.
  * Anything in `std::io`: replace with `rocket::tokio::io`.
  * Sleep or timers: replace with `rocket::tokio::time`.
  * Any networking: replace with `rocket::tokio::net`.
  * Any file system access: replace with `rocket::tokio::fs`.

Unfortunately, the Rust compiler provides no support for identifying blocking
I/O via lints or compile-time checks: it is up to you to scan your application
for sources of blocking I/O and replace them with their `async` counterpart. If
no such counterpart exists, you must execute the I/O in its own thread by using
[`rocket::tokio::task::spawn_blocking`].

All of Rocket's I/O APIs have been updated to be `async`-safe.
This results in requiring `.await` calls for common APIs like [`NamedFile`]. To
use `.await` in a route, the handler must be marked with `async`:

```rust
# use rocket::get;
use rocket::fs::NamedFile;

#[get("/")]
async fn index() -> Option<NamedFile> {
    NamedFile::open("index.html").await.ok()
}
```

<details>
<summary>See a diff of the changes from v0.4.</summary>

```diff
- use rocket::response::NamedFile;
+ use rocket::fs::NamedFile;

#[get("/")]
- fn index() -> Option<NamedFile> {
-    NamedFile::open("index.html").ok()
+ async fn index() -> Option<NamedFile> {
+    NamedFile::open("index.html").await.ok()
}
```

</details>

[`tokio`]: https://tokio.rs
[configurable number of worker threads]: ../configuration/#workers
[`NamedFile`]: @api/rocket/fs/struct.NamedFile.html
[`rocket::tokio::task::spawn_blocking`]: @tokio/task/fn.spawn_blocking.html

### Blocking Compute

By the same reasoning, performing large amounts of compute (really, just another
form of I/O) can prevent other tasks from executing in a timely manner. If you
are performing long computations in a handler, you should execute the
computation in its own thread, again using [`rocket::tokio::task::spawn_blocking`]:

```rust
# use rocket::get;
use rocket::tokio::task;
use rocket::response::Debug;

#[get("/")]
async fn exepensive() -> Result<(), Debug<task::JoinError>> {
    let result = task::spawn_blocking(move || {
        // perform the computation
    }).await?;

    Ok(result)
}
```

### Async Traits

To support `async` methods in traits, Rocket provides the [`async_trait`]
attribute. The attribute _must_ be applied to all implementations of _async
traits_ like [`FromRequest`] and [`Fairing`]:

```diff
use rocket::request::{self, Request, FromRequest};

+ #[rocket::async_trait]
impl<'r> FromRequest<'r> for MyType {
    type Error = MyError;

-    fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
+    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        /* .. */
    }
}
```

All trait documentation has been updated to call out such traits with an example
implementation that includes the invocation. The example implementation also
serves as better documentation for trait and trait method signatures than the
rustdocs. Because `async_trait` modifies these signatures, the rustdocs diverge
from what is written in source. For example, rustdoc renders:

```rust,ignore
fn from_request<'life0, 'async_trait>(
    request: &'r Request<'life0>
) -> Pin<Box<dyn Future<Output = Outcome<Self, Self::Error>> + Send + 'async_trait>>;
```

...whereas the source looks like:

```rust,ignore
async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error>;
```

Unfortunately, rustdoc does not provide a mechanism to render the source as it
is written. As such, we encourage all authors to use the examples as the source
of truth for trait and method signatures.

[`async_trait`]: @api/rocket/attr.async_trait.html
[`FromRequest`]: @api/rocket/request/trait.FromRequest.html
[`Fairing`]: @api/rocket/fairing/trait.Fairing.html

## Configuration

Rocket's configuration system has been entirely revamped for 0.5. The
[configuration](../configuration) section of the guide contains a full
walkthrough of the new system while the [general changes] section of the
CHANGELOG contains further details on configuration changes. We call out the
most important of these changes here. All users _must_:

  * Replace `ROCKET_ENV` environment variable use with `ROCKET_PROFILE`.
  * Replace `ROCKET_LOG` environment variable with `ROCKET_LOG_LEVEL`.
  * Use only IP addreses for the `address` configuration parameter.
  * Replace the `dev` or `development` profile with `debug`.
  * Note that the `stage`, `staging`, `prod`, and `production` profiles carry no
    special meaning in v0.5.
  * Use `0` to disable `keep_alive` instead of `false` or `off`.
  * Replace uses of "extras" with [typed extraction].

Rocket will emit warnings at launch time if use of the previous functionality is
detected.

### Profiles

The new system deals with "profiles" where there were previously "environments".
As opposed to environments, profiles:

  * Can be arbitrarily named, and any number can exist.
  * Match Rust profiles in naming: `debug` and `release` are the default
    profiles for the respective Rust compilation profile.
  * Are programmatically selectable and configurable.
  * Have a `default` profile with fallback values for all profiles.
  * Have a `global` profile with overrides for all profiles.

Authors should read the new [configuration](../configuration) section of the
guide to determine the scope of changes required. This likely includes:

  * Defining most configuration in the `default` profile instead.
  * Using the `debug` profile where `dev` or `development` was used.
  * Using the `release` profile where `prod` or `production` was used.

[general changes]: @github/CHANGELOG.md#general
[typed extraction]: ../configuration/#extracting-values

### Typed Extraction

The "extras" configuration in v0.4 is entirely replaced by [typed extraction],
which allows any `Deserialize` structure to be derived from configuration
sources. All users _should_ make use of typed extraction where "extras" were
being used previously. The diff below illustrates one such example:

```diff
use rocket::fairing::AdHoc;

+ #[derive(Deserialize)]
struct AppConfig {
    id: Option<usize>,
    port: u16,
}

- fn main() {
-     rocket::ignite()
-         .attach(AdHoc::on_attach("Token Config", |rocket| {
-             println!("Adding token managed state from config...");
-             let id = match rocket.config().get_int("id") {
-                 Ok(v) if v >= 0 => Some(v as usize),
-                 _ => None,
-             };
-
-             let port = match rocket.config().get_int("port") {
-                 Ok(v) if v => 0 && v < 1 << 16 => v as u16,
-                 _ => return Err(rocket)
-             };
-
-             Ok(rocket.manage(AppConfig { id, port }))
-         }))
- }

+ #[launch]
+ fn rocket() -> _ {
+     rocket::build().attach(AdHoc::config::<AppConfig>())
+ }
```

## Routing

Rocket v0.5 brings several major changes that affect routing:

  1. [Default ranking] is more precise, so fewer routes need manual ranking.
  2. Multi-segment route parameters (`<foo..>`) now match _zero_ or more
     segments.
  3. Parameters are _always_ percent-decoded, so `&RawStr` no longer implements
     `FromParam`.
  4. Query parameters parse with [`FromForm`] instead of `FromQuery` and support
     arbitrarily collections, nesting, structures, etc.
  5. All UTF-8 characters are allowed in static path components: `#[get("/❤️")]`.
  6. The [`register()`] method require a path to [scope catchers] under. Using
     `"/"` emulates the previous behavior.

[Default ranking]: ../requests#default-ranking
[`FromForm`]: @api/rocket/form/trait.FromForm.html
[`FromParam`]: @api/rocket/request/trait.FromParam.html
[`register()`]: @api/rocket/struct.Rocket.html#method.register
[scope catchers]: ../requests/#scoping

### Default Ranks

Default route ranking now takes into account partially dynamic paths, increasing
the range of default ranks from `[-6, -1]` to `[-12, -1]`. The net effect is
that fewer routes collide by default, requiring less manual ranking. For
example, the following two routes collide in 0.4 but not in 0.5:

```rust
# use rocket::get;

#[get("/foo/<_>/bar")]
fn foo_bar() { }

#[get("/<_..>")]
fn everything() { }
```

<details>
<summary>See a diff of the changes from v0.4.</summary>

```diff
- #[get("/foo/<_>/bar", rank = 1)]
+ #[get("/foo/<_>/bar")]
  fn foo_bar() { }

- #[get("/<_..>", rank = 2)]
+ #[get("/<_..>")]
  fn everything() { }
```
</details>

**The recommendation** is to remove all unnecessary manual ranking parameters.
For smaller applications, you may find that _all_ manual ranks can be removed.
Larger applications may still require ranks to resolve ambiguities.

### Kleen Multi-Segments

The multi-segment route parameter `<foo..>` now matches _zero or more_ segments,
a change from the previous _one_ or more segments. The implication is two-fold:

  1. Where previously two routes were required to match a prefix and its
     suffixes, now one suffices:

   ```diff
   - #[get("/")]
   - fn index();

   - #[get("/<path..>")]
   - fn rest(path: PathBuf);

   + #[get("/<path..>")]
   + fn all(path: PathBuf);
   ```

  2. A prefix collides with a route that matches all of its suffixes. For
     example, `index` and `rest` above collide.

Most applications will likely benefit from this change by allowing the extra
prefix-only route to be removed entirely. If the previous functionality of
requiring at least one segment is desired, a route that explicitly matches the
first segment can be used:

```rust
# use std::path::PathBuf;
# use rocket::get;

#[get("/<first>/<rest..>")]
fn rest(first: PathBuf, rest: PathBuf) { /* .. */ }
```

### Fewer Raw Strings

Rocket v0.5 makes a concerted effort to limit the exposure to strings from the
raw HTTP payload. In line with this philosophy, Rocket now percent-decodes all
incoming parameters automatically as opposed to doing so on-demand. The
corollary is three-fold:

  1. The `&RawStr` type no longer implements [`FromParam`].
  2. The `&str` type now implements [`FromParam`] and is fully decoded.
  3. The `String` parameter type is identical to the `&str` type and should be
     avoided.

Most applications can simply swap uses of `&RawStr` and `String` for `&str` in
routes, forms, and so on to benefit from the increase web-safety and
performance. For instance, the front-page example becomes:

```diff
 #[get("/<name>/<age>")]
- fn hello(name: String, age: u8) -> String {
+ fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}
```

A form that previously used `String` becomes:

```diff
#[derive(FromForm)]
- struct MyForm {
+ struct MyForm<'r> {
-    value: String,
+    value: &'r str,
}
```

### Queries as Forms

Query strings in Rocket v0.5 are in parity with forms and support their [full
breadth](../requests#forms). Single segment query parameters (`<foo>`) should
require little to no changes, except that they now support collections,
structures, and any other `FromForm` type. This implies that the majority, if
not _all_ custom `FromQuery` implementations, should be derivable via `FromForm`
or have a built-in equivalent like `Vec<T>`:

```rust
# use rocket::post;

#[post("/?<numbers>")]
fn form(numbers: Vec<usize>) { /* .. */ }
```

Multi-segment query parameters (`<foo..>`) no longer require the use of a
`Form<T>` guard. Instead, `T` can be used directly:

```diff
#[derive(FromForm)]
struct Person { /* .. */ }

#[get("/hello?<person..>")]
- fn hello(person: Option<Form<Person>>)
+ fn hello(person: Option<Person>)
```

## Forms

Rocket v0.5 introduces entirely revamped [forms] with support for:

  * [Multipart uploads.](../requests#multipart)
  * [Collections: maps, vectors, and more.](../requests#collections)
  * [Nesting.](../requests#nesting)
  * [Ad-Hoc validation.](../requests#ad-hoc-validation)

Additionally, the [`FromForm` derive] has been substantially improved so that
nearly all custom implementations of `FromForm` or (the now defunct)
`FromFormValue` can be derived. Altogether, this means that any external crate
dependency for form handling and most custom `FromForm` or `FromFormValue`
implementations are unnecessary and should be removed.

### Multipart

If your application used an external crate to accept multipart form submissions,
the dependency should be removed: Rocket v0.5 natively handles multipart. A file
upload can be accepted via the [`TempFile`] form guard:

```rust
# #[macro_use] extern crate rocket;

use rocket::form::Form;
use rocket::fs::TempFile;

#[derive(FromForm)]
struct Upload<'r> {
    save: bool,
    file: TempFile<'r>,
}

#[post("/upload", data = "<upload>")]
fn upload(upload: Form<Upload<'_>>) { /* .. */ }
```

[`TempFile`]: @api/rocket/fs/enum.TempFile.html

### Field Validation

In Rocket v0.4, it was encouraged and often required to implement
`FromFormValue` to introduce typed field validation. In v0.5, this can be
accomplished by [deriving `FromForm`]:

```diff
- use rocket::request::FromFormValue;
- use rocket::http::RawStr;
-
- struct AdultAge(usize);
-
- impl<'v> FromFormValue<'v> for AdultAge {
-     type Error = &'v RawStr;
-
-     fn from_form_value(form_value: &'v RawStr) -> Result<AdultAge, &'v RawStr> {
-         match form_value.parse::<usize>() {
-             Ok(age) if age >= 21 => Ok(AdultAge(age)),
-             _ => Err(form_value),
-         }
-     }
- }

+ #[derive(FromForm)]
+ #[field(validate = range(21..))]
+ struct AdultAge(usize);
```

If a given validation is used once, a new type may offer no additional safety.
The validation can be performed directly on a field:

```rust
use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    #[field(validate = range(21..))]
    age: usize,
}
```

[forms]: ../requests#forms
[`FromForm` derive]: @api/rocket/derive.FromForm.html
[deriving `FromForm`]: @api/rocket/derive.FromForm.html

## Notable New Features

Rocket v0.5 brings an abundance of new features that enable new functionality,
increase productivity, and make existing applications more robust. We encourage
all users to take advantage of these new features.

### Sentinels

Rocket v0.5 introduces [sentinels]. Entirely unique to Rocket, sentinels offer
an automatic last line of defense against runtime errors by enabling any type
that appears in a route to abort application launch if invalid conditions are
detected. For example, the [`&State<T>`] guard in v0.5 is a [`Sentinel`] that
aborts launch if the type `T` is not in managed state, thus preventing
associated runtime errors.

You should consider implementing `Sentinel` for your types if you have guards
(request, data, form, etc.) or responders that depend on `Rocket` state to
function properly. For example, consider a `MyResponder` that expects:

  * An error catcher to be registered for the `400` status code.
  * A specific type `T` to be in managed state.

Making `MyResponder` a sentinel that guards against these conditions is as
simple as:

```rust
use rocket::{Rocket, Ignite, Sentinel};
# struct MyResponder;
# struct T;

impl Sentinel for MyResponder {
    fn abort(r: &Rocket<Ignite>) -> bool {
        !r.catchers().any(|c| c.code == Some(400)) || r.state::<T>().is_none()
    }
}
```

[sentinels]: @api/rocket/trait.Sentinel.html
[`Sentinel`]: @api/rocket/trait.Sentinel.html
[`&State<T>`]: @api/rocket/struct.State.html

### More Typed URIs

Rocket v0.5 brings a completely overhauled [`uri!()`] macro and support for
typed URIs in more APIs. Notably, the `uri!()` macro now:

  * Allows URIs to be constructed from static values:

    ```rust
    # use rocket::uri;
    use rocket::http::uri::Absolute;

    const HOST: Absolute<'static> = uri!("http://localhost:8000");
    ```

  * Allows static and dynamic [prefixes and suffixes] to route URIs to be
    specified:

    ```rust
    # use rocket::{uri, get};

    #[get("/person/<name>?<age>")]
    fn person(name: &str, age: Option<u8>) { }

    let uri = uri!("https://rocket.rs/", person("Bob", Some(28)), "#woo");
    assert_eq!(uri.to_string(), "https://rocket.rs/person/Bob?age=28#woo");

    let host = uri!("http://bob.me");
    let uri = uri!(host, person("Bob", Some(28)));
    assert_eq!(uri.to_string(), "http://bob.me/person/Bob?age=28");
    ```

APIs like [`Redirect`] and [`Client`] now accept typed URIs:

```rust
# #[macro_use] extern crate rocket;

use rocket::response::Redirect;

#[get("/bye/<name>/<age>")]
fn bye(name: &str, age: u8) -> Redirect {
    Redirect::to(uri!("https://rocket.rs", bye(name, age), "?bye#now"))
}

#[test]
fn test() {
    use rocket::local::blocking::Client;

    let client = Client::new(rocket::build());
    let r = client.get(uri!(super::bye("Bob", 30))).dispatch();
}
```

[URI types] have been overhauled accordingly. A new [`Reference`] type encodes
URI-references. Additionally, all URI types are now `Serialize` and
`Deserialize`, allowing URIs to be used in configuration and passed over the
wire.

[`Redirect`]: @api/rocket/response/struct.Redirect.html
[`Client`]: @api/rocket/local/index.html
[prefixes and suffixes]: @api/rocket/macro.uri.html#prefixes-and-suffixes
[`uri!()`]: @api/rocket/macro.uri.html
[URI types]: @api/rocket/http/uri/index.html
[`Reference`]: @api/rocket/http/uri/struct.Reference.html

### Real-Time Streams

Rocket v0.5 introduces real-time, typed, `async` [streams]. The new [async
streams] section of the guide contains further details, and we encourage all
interested parties to see the new real-time, multi-room [chat example].

As a taste of what's possible, the following `stream` route emits a `"ping"`
Server-Sent Event every `n` seconds, defaulting to `1`:

```rust
# use rocket::*;
use rocket::response::stream::{Event, EventStream};;
use rocket::tokio::time::{interval, Duration};

#[get("/ping?<n>")]
fn stream(n: Option<u64>) -> EventStream![] {
    EventStream! {
        let mut timer = interval(Duration::from_secs(n.unwrap_or(1)));
        loop {
            yield Event::data("ping");
            timer.tick().await;
        }
    }
}
```

[streams]: @api/rocket/response/stream/index.html
[async streams]: ../responses/#async-streams
[chat example]: @example/chat

## Getting Help

If you run into any issues upgrading, we encourage you to ask questions via
[GitHub discussions] or via chat at [`#rocket:mozilla.org`] on Matrix or the
bridged [`#rocket`] IRC channel at `irc.libera.chat`.

[GitHub discussions]: https://github.com/SergioBenitez/Rocket/discussions
[`#rocket:mozilla.org`]: https://chat.mozilla.org/#/room/#rocket:mozilla.org
[`#rocket`]: https://kiwiirc.com/client/irc.libera.chat/#rocket
