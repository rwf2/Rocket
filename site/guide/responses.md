# Responses

You may have noticed that the return type of a handler appears to be arbitrary,
and that's because it is! A value of any type that implements the
[Responder](https://api.rocket.rs/rocket/response/trait.Responder.html) trait
can be returned, including your own.

## Responder

Types that implement `Responder` know how to generate a
[Response](https://api.rocket.rs/rocket/response/struct.Response.html) from
their values. A `Response` includes the HTTP status, headers, and body of the
response. Rocket implements `Responder` for many built-in types including
`String`, `&str`, `File`, `Option`, `Result`, and others. Rocket also provides
custom types, such as
[Content](https://api.rocket.rs/rocket/response/struct.Content.html) and
[Flash](https://api.rocket.rs/rocket/response/struct.Flash.html), which you can
find in the [response](https://api.rocket.rs/rocket/response/index.html) module.

The body of a `Response` may either be _fixed-sized_ or _streaming_. The given
`Responder` implementation decides which to use. For instance, `String` uses a
fixed-sized body, while `File` uses a streaming body.

### Wrapping

Responders can _wrap_ other responders. That is, responders can be of the
following form, where `R: Responder`:

```rust
struct WrappingResponder<R>(R);
```

When this is the case, the wrapping responder will modify the response returned
by `R` in some way before responding itself. For instance, to override the
status code of some response, you can use the types in the [status
module](https://api.rocket.rs/rocket/response/status/index.html). In particular,
to set the status code of a response for a `String` to **202 Accepted**, you can
return a type of `status::Accepted<String>`:

```rust
#[get("/")]
fn accept() -> status::Accepted<String> {
    status::Accepted(Some("I accept!".to_string()))
}
```

By default, the `String` responder sets the status to **200**. By using the
`Accepted` type however, The client will receive an HTTP response with status
code **202**.

Similarly, the types in the [content
module](https://api.rocket.rs/rocket/response/content/index.html) can be used to
override the Content-Type of the response. For instance, to set the Content-Type
of some `&'static str` to JSON, you can use the
[content::JSON](https://api.rocket.rs/rocket/response/content/struct.JSON.html)
type as follows:

```rust
#[get("/")]
fn json() -> content::JSON<&'static str> {
    content::JSON("{ 'hi': 'world' }")
}
```

## Errors

Responders need not _always_ generate a response. Instead, they can return an
`Err` with a given status code. When this happens, Rocket forwards the request
to the error catcher for the given status code. If none exists, which can only
happen when using custom status codes, Rocket uses the **500** error catcher.

### Result

`Result` is one of the most commonly used responders. Returning a `Result` means
one of two things. If the error type implements `Responder`, the `Ok` or `Err`
value will be used, whichever the variant is. If the error type does _not_
implement `Responder`, the error is printed to the console, and the request is
forwarded to the **500** error catcher.

### Option

`Option` is another commonly used responder. If the `Option` is `Some`, the
wrapped responder is used to respond to the client. Otherwise, the request is
forwarded to the **404** error catcher.

### Failure

While not encouraged, you can also forward a request to a catcher manually by
using the [Failure](https://api.rocket.rs/rocket/response/struct.Failure.html)
type. For instance, to forward to the catcher for **406 Not Acceptable**, you
would write:

```rust
#[get("/")]
fn just_fail() -> Failure {
    Failure(Status::NotAcceptable)
}
```

## JSON

Responding with JSON data is simple: return a value of type
[JSON](https://api.rocket.rs/rocket_contrib/struct.JSON.html). For example, to
respond with the JSON value of the `Task` structure from previous examples, we
would write:

```rust
#[derive(Serialize)]
struct Task { ... }

#[get("/todo")]
fn todo() -> JSON<Task> { ... }
```

The generic type in `JSON` must implement `Serialize`. The `JSON` type
serializes the structure into JSON, sets the Content-Type to JSON, and emits the
serialization in a fixed-sized body. If serialization fails, the request is
forwarded to the **500** error catcher.

For a complete example, see the [JSON example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/json).

## Templates

Rocket has built-in support for templating. To respond with a rendered template,
simply return a
[Template](https://api.rocket.rs/rocket_contrib/struct.Template.html) type.

```rust
#[get("/")]
fn index() -> Template {
  let context = /* object-like value */;
  Template::render("index", &context)
}
```

Templates are rendered with the `render` method. The method takes in the name of
a template and a context to render the template with. Rocket searches for a
template with that name in the configurable `template_dir` configuration
parameter, which defaults to `templates/`. Templating support in Rocket is
engine agnostic. The engine used to render a template depends on the template
file's extension. For example, if a file ends with `.hbs`, Handlebars is used,
while if a file ends with `.tera`, Tera is used.

The context can be any type that implements `Serialize` and serializes to an
`Object` value, such as structs, `HashMaps`, and others. The
[Template](https://api.rocket.rs/rocket_contrib/struct.Template.html) API
documentation contains more information about templates, while the [Handlebars
Templates example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/handlebars_templates)
is a fully composed application that makes use of Handlebars templates.

## Streaming

When a large amount of data needs to be sent to the client, it is better to
stream the data to the client to avoid consuming large amounts of memory. Rocket
provides the [Stream](https://api.rocket.rs/rocket/response/struct.Stream.html)
type, making this easy. The `Stream` type can be created from any `Read` type.
For example, to stream from a local Unix stream, we might write:

```rust
#[get("/stream")]
fn stream() -> io::Result<Stream<UnixStream>> {
    UnixStream::connect("/path/to/my/socket").map(|s| Stream::from(s))
}

```

Rocket takes care of the rest.
