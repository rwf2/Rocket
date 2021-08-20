# FAQ

Below you'll find a collection of commonly asked questions and answers. If you
have suggestions for questions you'd like to see answered here, [comment on the
discussion thread].

[comment on the discussion thread]: https://github.com/SergioBenitez/Rocket/discussions/1836

## About Rocket

<details id="monolithic">
<summary>
Is Rocket a monolithic framework like Rails? Or is it more like Flask?
<a class="headerlink" href="#monolithic" title="permalink">#</a>
</summary>
<div class="content">

Neither!

Rocket's core is small yet complete when it comes to security and correctness.
It mainly consists of:

  * Guard traits like [`FromRequest`] and [`FromData`].
  * Derive macros for all commonly used traits.
  * Attribute macros for routing.
  * Thorough compile and launch-time checking.
  * Optional features to enable functionality like TLS, secrets, and so on.
  * Zero-copy parsers and validators for common formats like multipart and SSE.
  * Syntax sugar extensions for features like async streams and traits.

The goal is for functionality like templating, sessions, ORMs, and so on to be
implemented entirely outside of Rocket and yet feel first-class. Indeed, crates
like [`rocket_dyn_templates`] and [`rocket_db_pools`] do just this.

As a result, Rocket is neither "bare-bones" nor is it a kitchen sink for all
possible features. Unlike other frameworks in the Rust ecosystem, Rocket makes
it its mission to help you avoid security and correctness blunders
out-of-the-box. It does this by including, out-of-the-box:

  * A flexible, type-based configuration system.
  * Security and privacy headers by default.
  * Zero-Copy RFC compliant URI parsers.
  * Safe, typed URIs with compile-time checking.
  * Thorough compile-time and launch-time checking of routes.
  * A complete testing framework with sync and `async` variants.
  * Safe, exclusive access to fully decoded HTTP values.
  * Mandatory data limits to prevent trivial DoS attacks.

Of course, this functionality comes at a compile-time cost (but notably, _not_
at a runtime cost), impacting Rocket's clean build-time. For comparison, here's
what building "Hello, world!" for the first time in popular Rust web frameworks
looks like:

| Framework            | Dependencies | Build Time |
|----------------------|--------------|------------|
| Rocket 0.5-rc.2      | 148          | 44s        |
| Actix-Web 4.0-beta.8 | 175          | 47s        |
| Tide 0.16            | 209          | 34s        |
| Warp 0.3             | 148          | 37s        |

<small>· Measurements taken on a MacBookPro15,1 Intel Core i9 @ 2.9GHZ, macOS
11.2.1, Rust 1.53 stable. Best of 3.</small><br />
<small>· Rocket includes features like multipart parsing and static file
serving that would require additional deps in other frameworks.</small>

Of course, iterative build-time is nearly identical for all frameworks, and the
time can be further reduced by using faster linkers like `lld`. We think the
trade-off is worth it. Rocket will never compromise security, correctness, or
usability to "win" at benchmarks of any sort.

</div>
</details>

[`rocket_dyn_templates`]: @api/rocket_dyn_templates
[`rocket_db_pools`]: @api/rocket_db_pools

<details id="compact">
<summary>
I want a small and compact web framework. Is Rocket it?
<a class="headerlink" href="#compact" title="permalink">#</a>
</summary>
<div class="content">

We think so! See ["Is Rocket a monolithic framework like Rails?"](#monolithic)
</div>
</details>

<details id="complete">
<summary>
I want a web framework with all the bells and whistles. Is Rocket it?
<a class="headerlink" href="#complete" title="permalink">#</a>
</summary>
<div class="content">

We think so! See ["Is Rocket a monolithic framework like Rails?"](#monolithic)
</div>
</details>

<details id="in-prod">
<summary>
Can I use Rocket in production? Should I? It's only v0.x!
<a class="headerlink" href="#in-prod" title="permalink">#</a>
</summary>
<div class="content">

We **enthusiastically** recommend using Rocket in production, with the following
caveats:

  1. Run Rocket behind a reverse proxy like HAProxy or in a production load
     balancing environment. Rocket (Hyper) doesn't employ any defenses against
     DDoS attacks or certain DoS attacks.

  2. Use a TLS termination proxy for zero-downtown certificate rotation.

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

</div>
</details>

<details id="performance">
<summary>
Is Rocket slow? Is Rocket fast?
<a class="headerlink" href="#performance" title="permalink">#</a>
</summary>
<div class="content">

Rocket is pretty fast.

A commonly repeated myth is that Rocket's great usability comes at the cost of
runtime performance. _**This is false.**_ Rocket's usability derives largely
from compile-time checks with _zero_ bearing on runtime performance.

So what about benchmarks? Well, benchmarking is _hard_, and besides often being
conducted incorrectly, often appear to say more than they do. So, when you see a
benchmark for "Hello, world!", you should know that the benchmark's relevance
doesn't extend far beyond "Hello, world!" servers and the specific way the
measurement was taken. In other words, they provide only a baseline truth that
is hard to extrapolate to real-world use-cases, _your_ use-case.

Nevertheless, here are some things you can consider as _generally_ true about
Rocket applications:

  * They'll perform much, _much_ better than those written in scripting
    languages like Python or Ruby.
  * They'll perform much better than those written in VM or JIT languages like
    JavaScript or Java.
  * They'll perform a bit better than those written in compiled but GC'd
    languages like Go.
  * They'll perform competitively with those written in compiled, non-GC'd
    languages like Rust or C.

Again, we emphasize _generally_ true. It is trivial to write a Rocket
application that is slower than a similar Python application.

Besides a framework's _internal_ performance, you should also consider whether
it enables your _application itself_ to perform well. Rocket takes great care to
enable your application to perform as little work as possible through
unique-to-Rocket features like [managed state], [request-local state], and
zero-copy parsing and deserialization.

</div>
</details>

[managed state]: ../state/#managed-state
[request-local state]: ../state/#request-local-state

<details id="showcase">
<summary>
What are some examples of "big" apps written in Rocket?
<a class="headerlink" href="#showcase" title="permalink">#</a>
</summary>
<div class="content">

Here are some notable projects and websites in Rocket we're aware of:

  * [Vaultwarden] - A BitWarden Server
  * [Conduit] - A Matrix Homeserver
  * [Rust-Lang.org] - Rust Language Website
  * [Plume] - Federated Blogging Engine
  * [Hagrid] - OpenPGP KeyServer ([keys.openpgp.org](https://keys.openpgp.org/))
  * [SourceGraph Syntax Highlighter] - Syntax Highlighting API

[Let us know] if you have a notable, public facing application written in Rocket
you'd like to see here!

[Vaultwarden]: https://github.com/dani-garcia/vaultwarden
[Conduit]: https://conduit.rs/
[Rust-Lang.org]: https://www.rust-lang.org/
[Plume]: https://github.com/Plume-org/Plume
[Hagrid]: https://gitlab.com/hagrid-keyserver/hagrid/
[SourceGraph Syntax Highlighter]: https://github.com/sourcegraph/syntect_server
[Let us know]: https://github.com/SergioBenitez/Rocket/discussions/categories/show-and-tell

</div>
</details>

<details id="releases">
<summary>
When will version `$y` be released? Why does it take so long?
<a class="headerlink" href="#releases" title="permalink">#</a>
</summary>
<div class="content">

Rocket represents an ecosystem-wide effort to create a web framework that
enables writing web applications with unparalleled security, performance, and
usability. From design to implementation to documentation, Rocket is carefully
crafted to ensure the greatest productivity and reliability with the fewest
surprises. Our goal is to make Rocket the obvious choice across _all_ languages.

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

</div>
</details>

## How To

<details id="web-sockets">
<summary>
Can I, and if so how, do I use WebSockets?
<a class="headerlink" href="#web-sockets" title="permalink">#</a>
</summary>
<div class="content">

Rocket doesn't support WebSockets quite yet. We're [working on it].

That being said, Rocket _does_ suport [Server-Sent Events], which allows for
real-time _unidirectional_ communication from the server to the client. This is
often sufficient for many of the applications that WebSockets are typically used
for. For instance, the [chat example] uses SSE to implement a real-time,
multiroom chat application.
</div>
</details>

[working on it]: https://github.com/SergioBenitez/Rocket/issues/90
[Server-Sent Events]: @api/rocket/response/stream/struct.EventStream.html
[chat example]: @example/chat

<details id="global-state">
<summary>
Should I use global state via something like `lazy_static!`?
<a class="headerlink" href="#global-state" title="permalink">#</a>
</summary>
<div class="content">

No. Rocket's [managed state] provides a better alternative.

While it may be convenient or comfortable to use global state, the downsides are
numerous. They include:

  * The inability to test your application with different state.
  * The inability to run your application on different threads with different
    state.
  * The inability to know the state a route accesses by looking at its
    signature.
</div>
</details>

[managed state]: ../state/#managed-state

<details id="file-uploads">
<summary>
How do I handle file uploads? What is this "multipart" in my stream?
<a class="headerlink" href="#file-uploads" title="permalink">#</a>
</summary>
<div class="content">

For a quick example on how to handle file uploads, see [multipart forms].

File uploads are transmitted by the browser as [multipart] form submissions,
which Rocket handles natively as a [`DataField`]. The [`TempFile`] form guard
can accept a `DataField` and stream the data to disk to then be persisted.
</div>
</details>

[multipart]: https://datatracker.ietf.org/doc/html/rfc7578
[multipart forms]: ../requests/#multipart
[`DataField`]: @api/rocket/form/struct.DataField.html
[`TempFile`]: @api/rocket/fs/enum.TempFile.html

<details id="raw-request">
<summary>
How do I get an `&Request` in a handler?
<a class="headerlink" href="#raw-request" title="permalink">#</a>
</summary>
<div class="content">

You don't!

Rocket's [philosophy] is that as much of the request should be validated and
converted into useful typed values _before_ being processed. Allowing a
`Request` to be handled directly is incompatible with this idea.

Instead, Rocket's handlers work through _guards_, reified as traits, which
validate and extract parts of a request as needed. Rocket automatically invokes
these guards for you, so custom guards are write-once-use-everywhere. Rocket
won't invoke handlers that depend on guards that fail. This way, handlers only
deal with fully validated, typed, secure values.

Rocket provides all of the guard implementations you would expect
out-of-the-box, and you can implement your own, too. See the following:

  * Parameter Guards: [`FromParam`]
  * Multi-Segment Guards: [`FromSegments`]
  * Data Guards: [`FromData`]
  * Form Guards: [`FromFrom`]
  * Request Guards: [`FromRequest`]
</div>
</details>

[philosophy]: ../introduction/#foreword
[`FromParam`]: @api/rocket/request/trait.FromParam.html
[`FromSegments`]: @api/rocket/request/trait.FromSegments.html
[`FromData`]: @api/rocket/data/trait.FromData.html
[`FromFrom`]: @api/rocket/form/trait.FromForm.html
[`FromRequest`]: @api/rocket/request/trait.FromRequest.html

<details id="response-headers">
<summary>
How do I add a header to a response?
<a class="headerlink" href="#response-headers" title="permalink">#</a>
</summary>
<div class="content">

That depends on the header!

Any "transport" headers (`Content-Length`, `Transfer-Encoding`, etc.) are
automatically set by Rocket and cannot be directly overridden for correctness
reasons. The rest are set by a type's [`Responder`] implementation.

**Status**

Rocket automatically sets a Status header for all responses. If the `Responder`
doesn't explicitly set a status, it defaults to `200`. Responders like
`Option<T>`, however, _do_ set the status. See the [`Responder`] docs for
details, and the [`status`] module for details on setting a custom Status or
overriding an existing one.

**Content-Type**

Rocket automatically sets a Content-Type header for most types it implements
`Responder` for, so in the common case, there's nothing to do. This includes
types like `&str`, `&[u8]`, `NamedFile`, and so on. The [`content`] module docs
have details on setting a custom Content-Type or overriding an existing one.

**Everything Else**

To add a custom header, you'll need a custom [`Responder`]. Not to worry!
[`Responder` can be derived](@api/rocket/derive.Responder.html) in almost all
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

You can always implement `Responder` directly. Make sure to leverage existing
responders in your implementation. For example, _don't_ serialize JSON manually.
Instead, use the existing [`Json`] responder, like in the example below:

```rust
# #[derive(rocket::serde::Serialize)]
# #[serde(crate = "rocket::serde")]
# struct Person { name: String, age: usize };

use rocket::request::Request;
use rocket::response::{self, Response, Responder};
use rocket::serde::json::Json;

impl<'r> Responder<'r, 'static> for Person {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(Json(&self).respond_to(req)?)
            .raw_header("X-Person-Name", self.name)
            .raw_header("X-Person-Age", self.age.to_string())
            .ok()
    }
}
```

</div>
</details>

[`Responder`]: @api/rocket/response/trait.Responder.html
[`content`]: @api/rocket/response/content/index.html
[`status`]: @api/rocket/response/status/index.html
[`Header`]: @api/rocket/http/struct.Header.html
[`Json`]: @api/rocket/serde/json/struct.Json.html

<details id="multiple-responses">
<summary>
How do I make one handler return different responses or status codes?
<a class="headerlink" href="#multiple-responses" title="permalink">#</a>
</summary>
<div class="content">

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

</div>
</details>

[`Either<A, B>`]: https://docs.rs/either/1/either/enum.Either.html
[derive a custom `Responder`]: @api/rocket/derive.Responder.html

<details id="automatic-reload">
<summary>
How do I make Rocket reload automatically when I change source code?
<a class="headerlink" href="#automatic-reload" title="permalink">#</a>
</summary>
<div class="content">

In debug mode, Rocket automatically reloads templates for you. So if all you
need is live template reloading, Rocket's got you covered.

For everything else, you'll need to use an external tool like [`cargo-watch`],
[`watchexec`] or [`entr`]. With `cargo-watch`, you can automatically rebuild and
run a Rocket application by executing:

```sh
cargo watch -x run
```

To only restart on successful compilations, see [this note].
</div>
</details>

[`cargo-watch`]: https://github.com/watchexec/cargo-watch
[`watchexec`]: https://github.com/watchexec/watchexec
[`entr`]: http://eradman.com/entrproject/
[this note]: https://github.com/watchexec/cargo-watch/tree/b75ce2c260874dea480f4accfd46ab28709ec56a#restarting-an-application-only-if-the-buildcheck-succeeds

<details id="external-managed-state">
<summary>
How do I access managed state outside of a Rocket-related context?
<a class="headerlink" href="#external-managed-state" title="permalink">#</a>
</summary>
<div class="content">

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

</div>
</details>

<details id="internal-server">
<summary>
How do I make Rocket a _part_ of my application as opposed to the whole thing?
<a class="headerlink" href="#internal-server" title="permalink">#</a>
</summary>
<div class="content">

If you're developing an application where an HTTP server is a _part_ of the
application instead of being the entire thing, use the `#[main]` attribute and
manually call [`launch()`]:


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

</div>
</details>

[`launch()`]: @api/rocket/struct.Rocket.html#method.launch

## Debugging

<details id="broken-example">
<summary>
Is example `foo` broken? It doesn't work for me.
<a class="headerlink" href="#broken-example" title="permalink">#</a>
</summary>
<div class="content">

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
</div>
</details>

<details id="unsat-bound">
<summary>
The trait bound `rocket::Responder` (`FromRequest`, etc.) is not satisfied.
<a class="headerlink" href="#unsat-bound" title="permalink">#</a>
</summary>
<div class="content">

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

...then you're almost certainly depending on libraries which depend on different
versions of `rocket`. A common mistake is to depend on a `contrib` library from
git while also depending on a `crates.io` version of Rocket or vice-versa:

```toml
rocket = "0.5.0-rc.1"
rocket_db_pools = { git = "https://github.com/SergioBenitez/Rocket.git" }
```

This is _never_ correct. In Rust, types from two different versions of a library
or from different providers (like `git` vs. `crates.io`) are _always_ considered
distinct, even if they have the same name. Therefore, even if a type implements
a trait from one library, it _does not_ implement the trait from the other
library (since it is considered to be a _different_, _distinct_ library). In
other words, you can _never_ mix two different published versions of Rocket, a
published version and a `git` version, or two instances from different `git`
revisions.

</div>
</details>
