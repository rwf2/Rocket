# Responses

You may have noticed that the return type of a handler appears to be arbitrary,
and that's because it is! A value of any type that implements the [`Responder`]
trait can be returned, including your own. In this section, we describe the
`Responder` trait as well as several useful `Responder`s provided by Rocket.
We'll also briefly discuss how to implement your own `Responder`.

[`Responder`]: https://api.rocket.rs/rocket/response/trait.Responder.html

## Responder

Types that implement [`Responder`] know how to generate a [`Response`] from
their values. A `Response` includes an HTTP status, headers, and body. The body
may either be _fixed-sized_ or _streaming_. The given `Responder` implementation
decides which to use. For instance, `String` uses a fixed-sized body, while
`File` uses a streamed response. Responders may dynamically adjust their
responses according to the incoming `Request` they are responding to.

[`Response`]: https://api.rocket.rs/rocket/response/struct.Response.html

### Wrapping

Before we describe a few responders, we note that it is typical for responders
to _wrap_ other responders. That is, responders can be of the following form,
where `R` is some type that implements `Responder`:

```rust
struct WrappingResponder<R>(R);
```

A wrapping responder modifies the response returned by `R` before responding
with that same response. For instance, Rocket provides `Responder`s in the
[`status` module](https://api.rocket.rs/rocket/response/status/index.html) that
override the status code of the wrapped `Responder`. As an example, the
[`Accepted`] type sets the status to `202 - Accepted`. It can be used as
follows:

```rust
use rocket::response::status;

#[post("/<id>")]
fn new(id: usize) -> status::Accepted<String> {
    status::Accepted(Some(format!("id: '{}'", id)))
}
```

Similarly, the types in the [`content`
module](https://api.rocket.rs/rocket/response/content/index.html) can be used to
override the Content-Type of a response. For instance, to set the Content-Type
an `&'static str` to JSON, you can use the [`content::Json`] type as follows:

```rust
use rocket::response::content;

#[get("/")]
fn json() -> content::Json<&'static str> {
    content::Json("{ 'hi': 'world' }")
}
```

[`Accepted`]: https://api.rocket.rs/rocket/response/status/struct.Accepted.html
[`content::Json`]: https://api.rocket.rs/rocket/response/content/struct.Json.html

### Errors

Responders may fail; they need not _always_ generate a response. Instead, they
can return an `Err` with a given status code. When this happens, Rocket forwards
the request to the [error catcher](/guide/requests/#error-catchers) for the
given status code.

If an error catcher has been registered for the given status code, Rocket will
invoke it. The catcher creates and returns a response to the client. If no error
catcher has been registered and the error status code is one of the standard
HTTP status code, a default error catcher will be used. Default error catchers
return an HTML page with the status code and description. If there is no catcher
for a custom status code, Rocket uses the **500** error catcher to return a
response.

While not encouraged, you can also forward a request to a catcher manually by
using the [`Failure`](https://api.rocket.rs/rocket/response/struct.Failure.html)
type. For instance, to forward to the catcher for **406 - Not Acceptable**, you
would write:

```rust
#[get("/")]
fn just_fail() -> Failure {
    Failure(Status::NotAcceptable)
}
```

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

`Option` is _wrapping_ responder: an `Option<T>` can only be returned when `T`
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
kinds of responses can be used depending on the circumstances. Revisting our
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
many of these responders in the [`response`] module. Among these are:

  * [`Content`] - Used to override the Content-Type of a response.
  * [`NamedFile`] - Streams a file to the client; automatically sets the
    Content-Type based on the file's extension.
  * [`Redirect`] - Redirects the client to a different URI.
  * [`Stream`] - Streams a response to a client from an arbitrary `Read`er type.
  * [`status`] - Contains types that override the status code of a response.
  * [`Flash`] - Sets a "flash" cookie that is removed when accessed.

[`status`]: https://api.rocket.rs/rocket/response/status/index.html
[`response`]: https://api.rocket.rs/rocket/response/index.html
[`NamedFile`]: https://api.rocket.rs/rocket/response/struct.NamedFile.html
[`Content`]: https://api.rocket.rs/rocket/response/struct.Content.html
[`Redirect`]: https://api.rocket.rs/rocket/response/struct.Redirect.html
[`Stream`]: https://api.rocket.rs/rocket/response/struct.Stream.html
[`Flash`]: https://api.rocket.rs/rocket/response/struct.Flash.html

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

[`rocket_contrib`]: https://api.rocket.rs/rocket_contrib/index.html

### JSON

The [`JSON`] responder in [`rocket_contrib`] allows you to easily respond with
well-formed JSON data: simply return a value of type `Json<T>` where `T` is the
type of a structure to serialize into JSON. The type `T` must implement the
[`Serialize`] trait from [`serde`], which can be automatically derived.

As an example, to respond with the JSON value of a `Task` structure, we might
write:

```rust
use rocket_contrib::Json;

#[derive(Serialize)]
struct Task { ... }

#[get("/todo")]
fn todo() -> Json<Task> { ... }
```

The `JSON` type serializes the structure into JSON, sets the Content-Type to
JSON, and emits the serialized data in a fixed-sized body. If serialization
fails, a **500 - Internal Server Error** is returned.

The [JSON example on GitHub] provides further illustration.

[`JSON`]: https://api.rocket.rs/rocket_contrib/struct.Json.html
[`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
[`serde`]: https://docs.serde.rs/serde/
[JSON example on GitHub]: https://github.com/SergioBenitez/Rocket/tree/v0.4.0-dev/examples/json

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

Rocket searches for a template with the given name in the configurable
`template_dir` directory. Templating support in Rocket is engine agnostic. The
engine used to render a template depends on the template file's extension. For
example, if a file ends with `.hbs`, Handlebars is used, while if a file ends
with `.tera`, Tera is used.

For templates to be properly registered, the template fairing must be attached
to the instance of Rocket. The [Fairings](/guide/fairings) sections of the guide
provides more information on fairings. To attach the template fairing, simply
call `.attach(Template::fairing())` on an instance of `Rocket` as follows:

```rust
fn main() {
    rocket::ignite()
      .mount("/", routes![...])
      .attach(Template::fairing());
}
```

The [`Template`] API
documentation contains more information about templates, while the [Handlebars
Templates example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.4.0-dev/examples/handlebars_templates)
is a fully composed application that makes use of Handlebars templates.

[`Template`]: https://api.rocket.rs/rocket_contrib/struct.Template.html
