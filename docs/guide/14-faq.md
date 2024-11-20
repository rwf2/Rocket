+++
summary = "answers to frequently asked questions about Rocket and its usage"
+++

{% macro faq(id) %}
<details id="{{ id }}">
<summary>
<a class="anchor" href="#{{ id }}" title="anchor">#</a>
{% endmacro %}

{% macro answer() %}
</summary>
<div class="content">
{% endmacro %}

{% macro endfaq() %}
</div>
</details>
{% endmacro %}

# FAQ

Below you'll find a collection of commonly asked questions and answers. If you
have suggestions for questions you'd like to see answered here, [comment on the
discussion thread].

[comment on the discussion thread]: https://github.com/rwf2/Rocket/discussions/1836

## About Rocket

{{ faq("monolithic") }}
Is Rocket a monolithic framework like Rails? Or is it more like Flask?
{{ answer() }}

Neither!

Rocket's core is small yet complete with respect to security and correctness. It
mainly consists of:

  * Guard traits like [`FromRequest`] and [`FromData`].
  * Derive macros for all common traits.
  * Attribute macros for routing.
  * Thorough compile and launch-time checking.
  * Zero-copy parsers and validators for common formats like multipart and SSE.
  * Syntax sugar extensions for features like async streams and traits.
  * Optional features for functionality like TLS, secrets, and so on.

The goal is for functionality like templating, sessions, ORMs, and so on to be
implemented entirely outside of Rocket while maintaining a first-class feel and
experience. Indeed, crates like [`rocket_dyn_templates`] and [`rocket_db_pools`]
do just this. As a result, Rocket is neither "bare-bones" nor is it a kitchen
sink for all possible features.

Unlike other frameworks, Rocket makes it its mission to help you avoid security
and correctness blunders. It does this by including, out-of-the-box:

  * A flexible, type-based [configuration](../configuration/) system.
  * [Security and privacy headers](@api/master/rocket/shield/) by default.
  * Zero-Copy RFC compliant [URI parsers](@api/master/rocket/http/uri).
  * Safe, [typed URIs](@api/master/rocket/macro.uri.html) with compile-time checking.
  * [Compile-time and launch-time route checking](@api/master/rocket/attr.route.html).
  * A [testing framework](@api/master/rocket/local) with sync and `async` variants.
  * Safe, exclusive access to fully decoded HTTP values.
  * Mandatory [data limits](@api/master/rocket/data/struct.Limits.html) to prevent
    trivial DoS attacks.

Of course, this functionality comes at a compile-time cost (but notably, _not_
a runtime cost), impacting Rocket's clean build-time. For comparison, here's
what a clean build of "Hello, world!" looks like for some Rust web frameworks:

| Framework       | Dependencies | Build Time | Build w/ `sscache` |
|-----------------|--------------|------------|--------------------|
| Rocket 0.5      | 105          | 12s        | 5s                 |
| Actix-Web 4.4.0 | 119          | 11s        | 4s                 |
| Axum 0.6.20     | 78           | 10s        | 4s                 |

<small>· Measurements taken on Apple Mac14,6 M2 Max, macOS 13, Rust 1.75. Best of 3.</small><br />
<small>· Rocket includes features like graceful shutdown, HTTP/2 keepalive, SSE
support, and static file serving that require additional deps in other frameworks.</small>

Of course, iterative build-time is nearly identical for all frameworks, and the
time can be further reduced by using faster linkers like `lld`. We think the
trade-off is worth it. Rocket will never compromise security, correctness, or
usability to "win" at benchmarks of any sort.

[`rocket_dyn_templates`]: @api/master/rocket_dyn_templates/
[`rocket_db_pools`]: @api/master/rocket_db_pools/
{{ endfaq() }}


{{ faq("compact") }}
I want a small and compact web framework. Is Rocket it?
{{ answer() }}
We think so! See ["Is Rocket a monolithic framework like Rails?"](#monolithic)
{{ endfaq() }}

{{ faq("complete") }}
I want a web framework with all the bells and whistles. Is Rocket it?
{{ answer() }}
We think so! See ["Is Rocket a monolithic framework like Rails?"](#monolithic)
{{ endfaq() }}

{{ faq("in-prod") }}
Can I use Rocket in production? Should I? It's only v0.x!
{{ answer() }}

We **enthusiastically** recommend using Rocket in production, with the following
non-exhaustive list of caveats:

  1. Run Rocket behind a reverse proxy like HAProxy or in a production load
     balancing environment. Rocket (Hyper) doesn't employ any defenses against
     DDoS attacks or certain DoS attacks which can be mitigated by an external
     service.

  2. Use a TLS termination proxy (perhaps from 1.) for zero-downtime certificate
     rotation.

  3. Properly configure your databases and database pools, especially with
     respect to the pool size.

  4. Ensure no blocking I/O happens outside of `spawn_blocking()` invocations.

While Rocket _is_ still in the `0.x` phase, the version number is purely a
stylistic choice. In fact, we consider Rocket to be the most mature web
framework in the Rust ecosystem. To our knowledge, Rocket is the only Rust web
framework that correctly implements:

  * Server-Sent Events
  * Graceful Shutdown
  * Form Parsing with Arbitrarily Structure
  * Zero-Copy, RFC Conforming URI Types
  * Ambiguity-Free Routing
  * Streamed Multipart Uploads

If you're coming from a different ecosystem, you should feel comfortable
considering Rocket's `v0.x` as someone else's `vx.0`. Rust and Cargo's semver
policy, and Rocket's strict adherence to it, ensures that Rocket will _never_
break your application when upgrading from `0.x.y` to `0.x.z`, where `z >= y`.
Furthermore, we backport _all_ security and correctness patches to the previous
major release (`0.{x-1}.y`), so your application remains secure if you need time
to upgrade.

{{ endfaq() }}

{{ faq("performance") }}
Is Rocket slow? Is Rocket fast?
{{ answer() }}

Rocket is pretty fast.

A commonly repeated myth is that Rocket's great usability comes at the cost of
runtime performance. _**This is false.**_ Rocket's usability derives largely
from compile-time checks with _zero_ bearing on runtime performance.

So what about benchmarks? Well, benchmarking is _hard_, and besides often being
conducted incorrectly<em>*</em>, often appear to say more than they do. So, when
you see a benchmark for "Hello, world!", you should know that the benchmark's
relevance doesn't extend far beyond those specific "Hello, world!" servers and
the specific way the measurement was taken. In other words, it provides _some_
baseline that is hard to extrapolate to real-world use-cases, _your_ use-case.

Nevertheless, here are some things you can consider as _generally_ true about
Rocket applications:

  * They'll perform much, _much_ better than those written in scripting
    languages like Python or Ruby.
  * They'll perform much better than those written in VM or JIT languages like
    JavaScript or Java.
  * They'll perform a bit better than those written in compiled-to-native but
    GC'd languages like Go.
  * They'll perform competitively with those written in compiled-to-native,
    non-GC'd languages like Rust or C.

Again, we emphasize _generally_ true. It is trivial to write a Rocket
application that is slower than a similar Python application.

Besides a framework's _internal_ performance, you should also consider whether
it enables your _application itself_ to perform well. Rocket takes great care to
enable your application to perform as little work as possible through
unique-to-Rocket features like [managed state], [request-local state], and
zero-copy parsing and deserialization.

<small>* A common mistake is to pit against Rocket's "Hello, world!" without
normalizing for response size, especially security headers.</small>

[managed state]: ../state/#managed-state
[request-local state]: ../state/#request-local-state
{{ endfaq() }}

{{ faq("showcase") }}
What are some examples of "big" apps written in Rocket?
{{ answer() }}

Here are some notable projects and websites in Rocket we're aware of:

  * [Vaultwarden] - A BitWarden Server
  * [Rust-Lang.org] - Rust Language Website
  * [Plume] - Federated Blogging Engine
  * [Hagrid] - OpenPGP KeyServer ([keys.openpgp.org](https://keys.openpgp.org/))
  * [SourceGraph Syntax Highlighter] - Syntax Highlighting API
  * [Revolt] - Open source user-first chat platform

[Let us know] if you have a notable, public facing application written in Rocket
you'd like to see here!

[Vaultwarden]: https://github.com/dani-garcia/vaultwarden
[Conduit]: https://conduit.rs/
[Rust-Lang.org]: https://www.rust-lang.org/
[Plume]: https://github.com/Plume-org/Plume
[Hagrid]: https://gitlab.com/keys.openpgp.org/hagrid
[SourceGraph Syntax Highlighter]: https://github.com/sourcegraph/sourcegraph/tree/main/docker-images/syntax-highlighter
[Let us know]: https://github.com/rwf2/Rocket/discussions/categories/show-and-tell
[Revolt]: https://github.com/revoltchat/backend
{{ endfaq() }}


{{ faq("releases") }}
When will version `$y` be released? Why does it take so long?
{{ answer() }}

Rocket represents an ecosystem-wide effort to create a web framework that
enables writing web applications with unparalleled security, performance, and
usability. From design to implementation to documentation, Rocket is carefully
crafted to ensure the greatest productivity and reliability with the fewest
surprises. Our goal is to make Rocket a compelling choice across _all_
languages.

Accomplishing this takes time, and our efforts extend to the entire ecosystem.
For example, work for Rocket v0.5 included:

  * [Fixing correctness issues in `x509-parser`.](https://github.com/rusticata/x509-parser/pull/90)
  * [Reporting multiple](https://github.com/bikeshedder/deadpool/issues/114)
    [correctness issues](https://github.com/bikeshedder/deadpool/issues/113) in `deadpool`.
  * [Fixing a major usability issue in `async-stream`.](https://github.com/tokio-rs/async-stream/pull/57)
  * [Creating a brand new configuration library.](https://github.com/SergioBenitez/Figment)
  * [Updating](https://github.com/rousan/multer-rs/pull/21),
    [fixing](https://github.com/rousan/multer-rs/pull/29), and
    [maintaining](https://github.com/rousan/multer-rs/commit/2758e778e6aa2785b737c82fe45e58026bea2f01) `multer`.
  * [Significantly improving `async_trait` correctness and usability.](https://github.com/dtolnay/async-trait/pull/143)
  * [Porting `Pattern` APIs to stable.](https://github.com/SergioBenitez/stable-pattern)
  * [Porting macro diagnostics to stable.](https://github.com/SergioBenitez/proc-macro2-diagnostics)
  * [Creating a brand new byte unit library.](https://github.com/SergioBenitez/ubyte)
  * [Fixing a bug in `rustc`'s `libtest`.](https://github.com/rust-lang/rust/pull/78227)

A version of Rocket is released whenever it is feature-complete and exceeds
feature, security, and usability parity with the previous version. As a result,
specifying a release date is nearly impossible. We are _always_ willing to delay
a release if these properties are not readily evident.

We know it can be frustrating, but we hope you'll agree that Rocket is worth the
wait.
{{ endfaq() }}

## How To

{{ faq("web-sockets") }}
Can I, and if so how, do I use WebSockets?
{{ answer() }}

You can! WebSocket support is provided by the officially maintained
[`rocket_ws`](@api/master/rocket_ws/) crate. You'll find all the docs you need
there.

Rocket _also_ supports [Server-Sent Events], which allows for real-time
_unidirectional_ communication from the server to the client. The protocol is a
bit simpler, and you may find SSE sufficient for your use-case. For instance,
the [chat example] uses SSE to implement a real-time, multiroom chat
application.

[Server-Sent Events]: @api/master/rocket/response/stream/struct.EventStream.html
[chat example]: @git/master/examples/chat
{{ endfaq() }}

{{ faq("global-state") }}
Should I use global state via something like `lazy_static!`?
{{ answer() }}

No. Rocket's [managed state] provides a better alternative.

While it may be convenient or comfortable to use global state, the downsides are
numerous. They include:

  * The inability to test your application with different state.
  * The inability to run your application on different threads with different
    state.
  * The inability to know the state a route accesses by looking at its
    signature.

[managed state]: ../state/#managed-state
{{ endfaq() }}

{{ faq("file-uploads") }}
How do I handle file uploads? What is this "multipart" in my stream?
{{ answer() }}

For a quick example on how to handle file uploads, see [multipart forms]. The
gist is: use `Form<TempFile>` as a data guard.

File uploads are encoded and transmitted by the browser as [multipart] forms.
The raw stream, as seen by [`Data`] for example, thus contains the necessary
metadata to encode the form. Rocket's [`Form`] data guard can parse these form
submissions into any type that implements [`FromForm`]. This includes types like
[`TempFile`] which streams the decoded data to disk for persistence.

[multipart]: https://datatracker.ietf.org/doc/html/rfc7578
[multipart forms]: ../requests/#multipart
[`DataField`]: @api/master/rocket/form/struct.DataField.html
[`TempFile`]: @api/master/rocket/fs/enum.TempFile.html
[`DataField`]: @api/master/rocket/data/struct.Data.html
[`Form`]: @api/master/rocket/form/struct.Form.html
[`FromForm`]: @api/master/rocket/form/trait.FromForm.html
[`Data`]: @api/master/rocket/struct.Data.html
{{ endfaq() }}

{{ faq("raw-request") }}
How do I get an `&Request` in a handler?
{{ answer() }}

You don't!

Rocket's [philosophy] is that as much of the request should be validated and
converted into useful typed values _before_ being processed. Allowing a
`Request` to be handled directly is incompatible with this idea.

Instead, Rocket's handlers work through _guards_, reified as traits, which
validate and extract parts of a request as needed. Rocket automatically invokes
these guards for you, so custom guards are write-once-use-everywhere. Rocket
won't invoke a handler with failing guards. This way, handlers only deal with
fully validated, typed, secure values.

Rocket provides all of the guard implementations you would expect
out-of-the-box, and you can implement your own, too. See the following:

  * Parameter Guards: [`FromParam`]
  * Multi-Segment Guards: [`FromSegments`]
  * Data Guards: [`FromData`]
  * Form Guards: [`FromForm`]
  * Request Guards: [`FromRequest`]

[philosophy]: ../introduction/#foreword
[`FromParam`]: @api/master/rocket/request/trait.FromParam.html
[`FromSegments`]: @api/master/rocket/request/trait.FromSegments.html
[`FromData`]: @api/master/rocket/data/trait.FromData.html
[`FromForm`]: @api/master/rocket/form/trait.FromForm.html
[`FromRequest`]: @api/master/rocket/request/trait.FromRequest.html
{{ endfaq() }}

{{ faq("response-headers") }}
How do I add a header to a response?
{{ answer() }}

That depends on the header!

Any "transport" headers (`Content-Length`, `Transfer-Encoding`, etc.) are
automatically set by Rocket and cannot be directly overridden for correctness
reasons. The rest are set by a route's [`Responder`].

**Status**

Rocket automatically sets a `Status` header for all responses. If a `Responder`
doesn't explicitly set a status, it defaults to `200`. Some responders, like
`Option<T>`, do set a status. See [`Responder`] and the [`status`] module for
details on setting a custom `Status` or overriding an existing one.

**Content-Type**

Rocket automatically sets a `Content-Type` header for types it implements
`Responder` for, so in the common case, there's nothing to do. This includes
types like `&str`, `&[u8]`, `NamedFile`, and so on. The [`content`] module docs
details setting a custom `Content-Type` or overriding an existing one.

**Everything Else**

To add a custom header, you'll need a custom [`Responder`]. Not to worry!
[`Responder` can be derived](@api/master/rocket/derive.Responder.html) in almost all
cases. If a type for the header you want to add already exists, you can directly
derive `Responder` for a struct that contains the header value, which adds the
header to the response:

```rust
# #[macro_use] extern crate rocket;
# use rocket::http::Header;

# type HeaderType = Header<'static>;

# impl<T> From<T> for MyResponder<T> {
#     fn from(inner: T) -> Self {
#         MyResponder { inner, header: Header::new("X-My-Header", "some value") }
#     }
# }

#[derive(Responder)]
struct MyResponder<T> {
    inner: T,
    header: HeaderType,
}

#[get("/")]
fn with_header() -> MyResponder<&'static str> {
    MyResponder::from("Hello, world!")
}
```

A `HeaderType` won't exist for custom headers, but you can define your own type.
As long as it implements `Into<Header>` for Rocket's [`Header`], the type can be
used as a field in derived struct.

Alternatively, you can always implement `Responder` directly. Make sure to
leverage existing responders in your implementation. For example, _don't_
serialize JSON manually. Instead, use the existing [`Json`] responder, like in
the example below:

```rust
# #[derive(rocket::serde::Serialize)]
# #[serde(crate = "rocket::serde")]
# struct Person { name: String, age: usize };

use rocket::request::Request;
use rocket::response::{self, Response, Responder};
use rocket::serde::json::Json;

impl<'r> Responder<'r, 'static> for Person {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r, 'static> {
        Response::build_from(Json(&self).respond_to(req)?)
            .raw_header("X-Person-Name", self.name)
            .raw_header("X-Person-Age", self.age.to_string())
            .ok()
    }
}
```

[`Responder`]: @api/master/rocket/response/trait.Responder.html
[`content`]: @api/master/rocket/response/content/index.html
[`status`]: @api/master/rocket/response/status/index.html
[`Header`]: @api/master/rocket/http/struct.Header.html
[`Json`]: @api/master/rocket/serde/json/struct.Json.html
{{ endfaq() }}

{{ faq("multiple-responses") }}
How do I make one handler return different responses or status codes?
{{ answer() }}

If you're returning _two_ different responses, use a `Result<T, E>` or an
[`Either<A, B>`].

If you need to return _more_ than two kinds, [derive a custom `Responder`] `enum`:

```rust
# use rocket::response::Responder;
use rocket::fs::NamedFile;
use rocket::http::ContentType;

#[derive(Responder)]
enum Error<'r, T> {
    #[response(status = 400)]
    Unauthorized(T),
    #[response(status = 404)]
    NotFound(NamedFile),
    #[response(status = 500)]
    A(&'r str, ContentType),
}
```

[`Either<A, B>`]: https://docs.rs/either/1/either/enum.Either.html
[derive a custom `Responder`]: @api/master/rocket/derive.Responder.html
{{ endfaq() }}

{{ faq("automatic-reload") }}
How do I make Rocket reload automatically when I change source code?
{{ answer() }}

In debug mode, Rocket automatically reloads templates for you. So if all you
need is live template reloading, Rocket's got you covered.

For everything else, you'll need to use an external tool like [`cargo-watch`],
[`watchexec`] or [`entr`]. With `cargo-watch`, you can automatically rebuild and
run a Rocket application by executing:

```sh
cargo watch -x run
```

To only restart on successful compilations, see [this note].

[`cargo-watch`]: https://github.com/watchexec/cargo-watch
[`watchexec`]: https://github.com/watchexec/watchexec
[`entr`]: http://eradman.com/entrproject/
[this note]: https://github.com/watchexec/cargo-watch/tree/b75ce2c260874dea480f4accfd46ab28709ec56a#restarting-an-application-only-if-the-buildcheck-succeeds
{{ endfaq() }}

{{ faq("external-managed-state") }}
How do I access managed state outside of a Rocket-related context?
{{ answer() }}

Use an `Arc`, like this:

```rust
# use rocket::*;
use std::sync::Arc;

#[launch]
fn rocket() -> _ {
    # struct MyState;
    let state = Arc::new(MyState);

    let external = state.clone();
    std::thread::spawn(move || {
        let use_state = external;
    });

    rocket::build().manage(state)
}
```

{{ endfaq() }}

{{ faq("internal-server") }}
How do I make Rocket a _part_ of my application as opposed to the whole thing?
{{ answer() }}

Use the `#[main]` attribute and manually call [`launch()`]:

```rust,no_run
#[rocket::main]
async fn main() {
    # let should_start_server = false;
    if should_start_server {
        let result = rocket::build().launch().await;
    } else {
        // do something else
    }
}
```

The cost to using the attribute is imperceptible and guarantees compatibility
with Rocket's async I/O.

[`launch()`]: @api/master/rocket/struct.Rocket.html#method.launch
{{ endfaq() }}

## Debugging

{{ faq("broken-example") }}
Is example `foo` broken? It doesn't work for me.
{{ answer() }}

Almost certainly not.

Every example and code snippet you see in published documentation is tested by
the CI on every commit, and we only publish docs that pass the CI. Unless the CI
environment is broken, the examples _cannot_ be wrong.

Common mistakes when running examples include:

  * Looking at an example for version `y` but depending on version `x`. Select
    the proper git tag!
  * Looking at outdated examples on StackOverflow or Google. Check the
    date/version!
  * Not configuring the correct dependencies. See the example's `Cargo.toml`!
{{ endfaq() }}

{{ faq("unsat-bound") }}
The trait bound `rocket::Responder` (`FromRequest`, etc.) is not satisfied.
{{ answer() }}

If you're fairly certain a type implements a given Rocket trait but still get an
error like:

```rust,ignore
error[E0277]: the trait bound `Foo: Responder<'_, '_>` is not satisfied
--> src\main.rs:4:20
|
4 | fn foo() -> Foo
|             ^^^ the trait `Responder<'_, '_>` is not implemented for `Foo`
|
= note: required by `respond_to`
```

...then you're almost certainly depending, perhaps transitively, on _two
different versions_ of a single library. For example, you may be depending on
`rocket` which depends on `time 0.3` while also depending directly on `time
0.2`. Or you may depending on `rocket` from `crates.io` while depending on a
library that depends on `rocket` from `git`. A common instance of this mistake
is to depend on a `contrib` library from git while also depending on a
`crates.io` version of Rocket or vice-versa:

```toml
rocket = "0.6.0-dev"
rocket_db_pools = { git = "https://github.com/rwf2/Rocket.git" }
```

This is _never_ correct. If libraries or applications interact via types from a
common library, those libraries or applications _must_ specify the _same_
version of that common library. This is because in Rust, types from two
different versions of a library or from different providers (like `git` vs.
`crates.io`) are _always_ considered distinct, even if they have the same name.
Therefore, even if a type implements a trait from one library, it _does not_
implement the trait from the other library (since it is considered to be a
_different_, _distinct_ library). In other words, you can _never_ mix two
different published versions of Rocket, a published version and a `git` version,
or two instances from different `git` revisions.
{{ endfaq() }}
