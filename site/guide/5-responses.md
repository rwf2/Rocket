# Responses

You may have noticed that the return type of a handler appears to be arbitrary,
and that's because it is! A value of any type that implements the [`Responder`]
trait can be returned, including your own. In this section, we describe the
`Responder` trait as well as several useful `Responder`s provided by Rocket.
We'll also briefly discuss how to implement your own `Responder`.

[`Responder`]: @api/rocket/response/trait.Responder.html

## Responder

Types that implement [`Responder`] know how to generate a [`Response`] from
their values. A `Response` includes an HTTP status, headers, and body. The body
may either be _fixed-sized_ or _streaming_. The given `Responder` implementation
decides which to use. For instance, `String` uses a fixed-sized body, while
`File` uses a streamed response. Responders may dynamically adjust their
responses according to the incoming `Request` they are responding to.

[`Response`]: @api/rocket/response/struct.Response.html

### Wrapping

Before we describe a few responders, we note that it is typical for responders
to _wrap_ other responders. That is, responders can be of the following form,
where `R` is some type that implements `Responder`:

```rust
struct WrappingResponder<R>(R);
```

A wrapping responder modifies the response returned by `R` before responding
with that same response. For instance, Rocket provides `Responder`s in the
[`status` module](@api/rocket/response/status/) that override the status code of
the wrapped `Responder`. As an example, the [`Accepted`] type sets the status to
`202 - Accepted`. It can be used as follows:

```rust
use rocket::response::status;

#[post("/<id>")]
fn new(id: usize) -> status::Accepted<String> {
    status::Accepted(Some(format!("id: '{}'", id)))
}
```

Similarly, the types in the [`content` module](@api/rocket/response/content/)
can be used to override the Content-Type of a response. For instance, to set the
Content-Type of `&'static str` to JSON, you can use the [`content::Json`] type
as follows:

```rust
use rocket::response::content;

#[get("/")]
fn json() -> content::Json<&'static str> {
    content::Json("{ 'hi': 'world' }")
}
```

! warning: This is _not_ the same as the [`Json`] in [`rocket_contrib`]!

[`Accepted`]: @api/rocket/response/status/struct.Accepted.html
[`content::Json`]: @api/rocket/response/content/struct.Json.html

### Errors

Responders may fail; they need not _always_ generate a response. Instead, they
can return an `Err` with a given status code. When this happens, Rocket forwards
the request to the [error catcher](../requests/#error-catchers) for the
given status code.

If an error catcher has been registered for the given status code, Rocket will
invoke it. The catcher creates and returns a response to the client. If no error
catcher has been registered and the error status code is one of the standard
HTTP status code, a default error catcher will be used. Default error catchers
return an HTML page with the status code and description. If there is no catcher
for a custom status code, Rocket uses the **500** error catcher to return a
response.

While not encouraged, you can also forward a request to a catcher manually by
using the [`Failure`](@api/rocket/response/struct.Failure.html)
type. For instance, to forward to the catcher for **406 - Not Acceptable**, you
would write:

```rust
use rocket::response::Failure;

#[get("/")]
fn just_fail() -> Failure {
    Failure(Status::NotAcceptable)
}
```

## Custom Responders

The [`Responder`] trait documentation details how to implement your own custom
responders by explicitly implementing the trait. For most use cases, however,
Rocket makes it possible to automatically derive an implementation of
`Responder`. In particular, if your custom responder wraps an existing
responder, headers, or sets a custom status or content-type, `Responder` can be
automatically derived:

```rust
#[derive(Responder)]
#[response(status = 500, content_type = "json")]
struct MyResponder {
    inner: OtherResponder,
    header: SomeHeader,
    more: YetAnotherHeader,
    #[response(ignore)]
    unrelated: MyType,
}
```

For the example above, Rocket generates a `Responder` implementation that:

  * Set the response's status to `500: Internal Server Error`.
  * Sets the Content-Type to `application/json`.
  * Adds the headers `self.header` and `self.more` to the response.
  * Completes the response using `self.inner`.

Note that the _first_ field is used as the inner responder while all remaining
fields (unless ignored with `#[response(ignore)]`) are added as headers to the
response. The optional `#[responder]` attribute can be used to customize the
status and content-type of the response. Because `ContentType` and `Status` are
themselves headers, you can also dynamically set the content-type and status by
simply including fields of these types.

For more on using the `Responder` derive, see the [`Responder` derive]
documentation.

[`Responder` derive]: @api/rocket_codegen/derive.Responder.html

## Implementations

Rocket implements `Responder` for many types in Rust's standard library
including `String`, `&str`, `File`, `Option`, and `Result`. The [`Responder`]
documentation describes these in detail, but we briefly cover a few here.

### Strings

The `Responder` implementations for `&str` and `String` are straight-forward:
the string is used as a sized body, and the Content-Type of the response is set
to `text/plain`. To get a taste for what such a `Responder` implementation looks
like, here's the implementation for `String`:

```rust
impl Responder<'static> for String {
    fn respond_to(self, _: &Request) -> Result<Response<'static>, Status> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self))
            .ok()
    }
}
```

Because of these implementations, you can directly return an `&str` or `String`
type from a handler:

```rust
#[get("/string")]
fn handler() -> &'static str {
    "Hello there! I'm a string!"
}
```

### `Option`

`Option` is a _wrapping_ responder: an `Option<T>` can only be returned when `T`
implements `Responder`. If the `Option` is `Some`, the wrapped responder is used
to respond to the client. Otherwise, a error of **404 - Not Found** is returned
to the client.

This implementation makes `Option` a convenient type to return when it is not
known until process-time whether content exists. For example, because of
`Option`, we can implement a file server that returns a `200` when a file is
found and a `404` when a file is not found in just 4, idiomatic lines:

```rust
#[get("/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}
```

### `Result`

`Result` is a special kind of wrapping responder: its functionality depends on
whether the error type `E` implements `Responder`.

When the error type `E` implements `Responder`, the wrapped `Responder` in `Ok`
or `Err`, whichever it might be, is used to respond to the client. This means
that the responder can be chosen dynamically at run-time, and two different
kinds of responses can be used depending on the circumstances. Revisiting our
file server, for instance, we might wish to provide more feedback to the user
when a file isn't found. We might do this as follows:

```rust
use rocket::response::status::NotFound;

#[get("/<file..>")]
fn files(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|_| NotFound(format!("Bad path: {}", path)))
}
```

If the error type `E` _does not_ implement `Responder`, then the error is simply
logged to the console, using its `Debug` implementation, and a `500` error is
returned to the client.

## Rocket Responders

Some of Rocket's best features are implemented through responders. You can find
many of these responders in the [`response`] module and [`rocket_contrib`]
library. Among these are:

  * [`Content`] - Used to override the Content-Type of a response.
  * [`NamedFile`] - Streams a file to the client; automatically sets the
    Content-Type based on the file's extension.
  * [`Redirect`] - Redirects the client to a different URI.
  * [`Stream`] - Streams a response to a client from an arbitrary `Read`er type.
  * [`status`] - Contains types that override the status code of a response.
  * [`Flash`] - Sets a "flash" cookie that is removed when accessed.
  * [`Json`] - Automatically serializes values into JSON.
  * [`MsgPack`] - Automatically serializes values into MessagePack.
  * [`Template`] - Renders a dynamic template using handlebars or Tera.

[`status`]: @api/rocket/response/status/
[`response`]: @api/rocket/response/
[`NamedFile`]: @api/rocket/response/struct.NamedFile.html
[`Content`]: @api/rocket/response/struct.Content.html
[`Redirect`]: @api/rocket/response/struct.Redirect.html
[`Stream`]: @api/rocket/response/struct.Stream.html
[`Flash`]: @api/rocket/response/struct.Flash.html
[`MsgPack`]: @api/rocket_contrib/msgpack/struct.MsgPack.html

### Streaming

The `Stream` type deserves special attention. When a large amount of data needs
to be sent to the client, it is better to stream the data to the client to avoid
consuming large amounts of memory. Rocket provides the [`Stream`] type, making
this easy. The `Stream` type can be created from any `Read` type. For example,
to stream from a local Unix stream, we might write:

```rust
#[get("/stream")]
fn stream() -> io::Result<Stream<UnixStream>> {
    UnixStream::connect("/path/to/my/socket").map(|s| Stream::from(s))
}

```

[`rocket_contrib`]: @api/rocket_contrib/

### JSON

The [`Json`] responder in [`rocket_contrib`] allows you to easily respond with
well-formed JSON data: simply return a value of type `Json<T>` where `T` is the
type of a structure to serialize into JSON. The type `T` must implement the
[`Serialize`] trait from [`serde`], which can be automatically derived.

As an example, to respond with the JSON value of a `Task` structure, we might
write:

```rust
use rocket_contrib::json::Json;

#[derive(Serialize)]
struct Task { ... }

#[get("/todo")]
fn todo() -> Json<Task> { ... }
```

The `Json` type serializes the structure into JSON, sets the Content-Type to
JSON, and emits the serialized data in a fixed-sized body. If serialization
fails, a **500 - Internal Server Error** is returned.

The [JSON example on GitHub] provides further illustration.

[`Json`]: @api/rocket_contrib/json/struct.Json.html
[`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
[`serde`]: https://docs.serde.rs/serde/
[JSON example on GitHub]: @example/json

### Templates

Rocket includes built-in templating support that works largely through a
[`Template`] responder in `rocket_contrib`. To render a template named "index",
for instance, you might return a value of type `Template` as follows:

```rust
#[get("/")]
fn index() -> Template {
    let context = /* object-like value */;
    Template::render("index", &context)
}
```

Templates are rendered with the `render` method. The method takes in the name of
a template and a context to render the template with. The context can be any
type that implements `Serialize` and serializes into an `Object` value, such as
structs, `HashMaps`, and others.

Rocket searches for a template with the given name in the [configurable]
`template_dir` directory. Templating support in Rocket is engine agnostic. The
engine used to render a template depends on the template file's extension. For
example, if a file ends with `.hbs`, Handlebars is used, while if a file ends
with `.tera`, Tera is used.

When your application is compiled in `debug` mode (without the `--release` flag
passed to `cargo`), templates are automatically reloaded when they are modified.
This means that you don't need to rebuild your application to observe template
changes: simply refresh! In release builds, reloading is disabled.

For templates to be properly registered, the template fairing must be attached
to the instance of Rocket. The [Fairings](../fairings) sections of the guide
provides more information on fairings. To attach the template fairing, simply
call `.attach(Template::fairing())` on an instance of `Rocket` as follows:

```rust
fn main() {
    rocket::ignite()
      .mount("/", routes![...])
      .attach(Template::fairing());
}
```

The [`Template`] API documentation contains more information about templates,
including how to customize a template engine to add custom helpers and filters.
The [Handlebars Templates example on GitHub](@example/handlebars_templates) is a
fully composed application that makes use of Handlebars templates.

[`Template`]: @api/rocket_contrib/templates/struct.Template.html
[configurable]: ../configuration/#extras

## Typed URIs

Rocket's [`uri!`] macro allows you to build URIs routes in your application in a
robust, type-safe, and URI-safe manner. Type or route parameter mismatches are
caught at compile-time.

The `uri!` returns an [`Origin`] structure with the URI of the supplied route
interpolated with the given values. Note that `Origin` implements `Into<Uri>`
(and by extension, `TryInto<Uri>`), so it can be converted into a [`Uri`] using
`.into()` as needed and passed into methods such as [`Redirect::to()`].

For example, given the following route:

```rust
#[get("/person/<name>/<age>")]
fn person(name: String, age: u8) -> String {
    format!("Hello {}! You're {} years old.", name, age)
}
```

URIs to `person` can be created as follows:

```rust
// with unnamed parameters, in route path declaration order
let mike = uri!(person: "Mike Smith", 28);
assert_eq!(mike.path(), "/person/Mike%20Smith/28");

// with named parameters, order irrelevant
let mike = uri!(person: name = "Mike", age = 28);
let mike = uri!(person: age = 28, name = "Mike");
assert_eq!(mike.path(), "/person/Mike/28");

// with a specific mount-point
let mike = uri!("/api", person: name = "Mike", age = 28);
assert_eq!(mike.path(), "/api/person/Mike/28");
```

Rocket informs you of any mismatched parameters at compile-time:

```rust
error: person route uri expects 2 parameter but 1 was supplied
  --> src/main.rs:21:19
   |
21 |     uri!(person: "Mike Smith");
   |                  ^^^^^^^^^^^^
   |
   = note: expected parameter: age: u8
```

Rocket also informs you of any type errors at compile-time:

```rust
error: the trait bound u8: rocket::http::uri::FromUriParam<&str> is not satisfied
  --> src/main:25:23
   |
25 |     uri!(person: age = "ten", name = "Mike");
   |                        ^^^^^ FromUriParam<&str> is not implemented for u8
   |
   = note: required by rocket::http::uri::FromUriParam::from_uri_param
```

We recommend that you use `uri!` exclusively when constructing URIs to your
routes.

See the [`uri!`] documentation for more usage details.

[`Origin`]: @api/rocket/http/uri/struct.Origin.html
[`Uri`]: @api/rocket/http/uri/enum.Uri.html
[`Redirect::to()`]: @api/rocket/response/struct.Redirect.html#method.to
[`uri!`]: @api/rocket_codegen/macro.uri.html
