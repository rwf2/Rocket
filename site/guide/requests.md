# Requests

If all we could do was match against static paths like `"/world"`, Rocket
wouldn't be much fun. Of course, Rocket allows you to match against just about
any information in an incoming request. This section describes the available
options and their effect on the application.

## Methods

A Rocket route attribute can be any one of `get`, `put`, `post`, `delete`,
`head`, `patch`, or `options`, each corresponding to the HTTP method to match
against. For example, the following attribute will match against `POST` requests
to the root path:

```rust
#[post("/")]
```

The grammar for these routes is defined formally in the
[rocket_codegen](https://api.rocket.rs/rocket_codegen/) API docs.

Rocket handles `HEAD` requests automatically when there exists a `GET` route
that would otherwise match. It does this by stripping the body from the
response, if there is one. You can also specialize the handling of a `HEAD`
request by declaring a route for it; Rocket won't interfere with `HEAD` requests
your application handles.

Because browsers only send `GET` and `POST` requests, Rocket _reinterprets_
requests under certain conditions. If a `POST` request contains a body of
`Content-Type: application/x-www-form-urlencoded`, and the form's **first**
field has the name `_method` and a valid HTTP method as its value, that field's
value is used as the method for the incoming request. This allows Rocket
applications to submit non-`POST` forms. The [todo
example](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/todo/static/index.html.tera#L47)
makes use of this feature to submit `PUT` and `DELETE` requests from a web form.

## Format

When receiving data, you can specify the Content-Type the route matches against
via the `format` route parameter. The parameter is a string of the Content-Type
expected. For example, to match `application/json` data, a route can be declared
as:

```rust
#[post("/user", format = "application/json", data = "<user>")]
fn new_user(user: JSON<User>) -> T { ... }
```

Note the `format` parameter in the `post` attribute. The `data` parameter is
described later in the [data](#data) section.

## Dynamic Paths

You can declare path segments as dynamic by using angle brackets around variable
names in a route's path. For example, if we wanted to say _Hello!_ to anything,
not just the world, we could declare a route and handler like so:

```rust
#[get("/hello/<name>")]
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

If we were to mount the path at the root (`.mount("/", routes![hello])`), then
any request to a path with two non-empty segments, where the first segment is
`hello`, will be dispatched to the `hello` route. For example, if we were to
visit `/hello/John`, the application would respond with `Hello, John!`.

You can have any number of dynamic path segments, and the type of the path
segment can be any type that implements the [FromParam
trait](https://api.rocket.rs/rocket/request/trait.FromParam.html), including
your own! Rocket implements `FromParam` for many of the standard library types,
as well as a few special Rocket types. Here's a somewhat complicated route to
illustrate varied usage:

```rust
#[get("/hello/<name>/<age>/<cool>")]
fn hello(name: &str, age: u8, cool: bool) -> String {
    if cool {
      format!("You're a cool {} year old, {}!", age, name)
    } else {
      format!("{}, we need to talk about your coolness.", name)
    }
}
```

## Forwarding

In this example above, what if `cool` isn't a `bool`? Or, what if `age` isn't a
`u8`? In this case, the request is _forwarded_ to the next matching route, if
there is any. This continues until a route doesn't forward the request or there
are no remaining routes to try. When there are no remaining matching routes, a
customizable **404 error** is returned.

Routes are tried in increasing _rank_ order. By default, routes with static
paths have a rank of 0 and routes with dynamic paths have a rank of 1. A route's
rank can be manually set with the `rank` route parameter.

To illustrate, consider the following routes:

```rust
#[get("/user/<id>")]
fn user(id: usize) -> T { ... }

#[get("/user/<id>", rank = 2)]
fn user_int(id: isize) -> T { ... }

#[get("/user/<id>", rank = 3)]
fn user_str(id: &str) -> T { ... }
```

Notice the `rank` parameters in `user_int` and `user_str`. If we run this
application with the routes mounted at the root, requests to `/user/<id>` will
be routed as follows:

  1. The `user` route matches first. If the string at the `<id>` position is an
     unsigned integer, then the `user` handler is called. If it is not, then the
     request is forwarded to the next matching route: `user_int`.

  2. The `user_int` route matches next. If `<id>` is a signed integer,
     `user_int` is called. Otherwise, the request is forwarded.

  3. The `user_str` route matches last. Since `<id>` is a always string, the
     route always matches. The `user_str` handler is called.

Forwards can be _caught_ by using a `Result` or `Option` type. For example, if
the type of `id` in the `user` function was `Result<usize, &str>`, then `user`
would never forward. An `Ok` variant would indicate that `<id>` was a valid
`usize`, while an `Err` would indicate that `<id>` was not a `usize`. The
`Err`'s value would contain the string that failed to parse as a `usize`.

By the way, if you were to omit the `rank` parameter in the `user_str` or
`user_int` routes, Rocket would emit a warning indicating that the routes
_collide_, or can match against similar incoming requests. The `rank` parameter
resolves this collision.

## Dynamic Segments

You can also match against multiple segments by using `<param..>` in the route
path. The type of such parameters, known as _segments_ parameters, can be any
that implements
[FromSegments](https://api.rocket.rs/rocket/request/trait.FromSegments.html).
Segments parameters must be the final component of the path: any text after a
segments parameter in a path will result in a compile-time error.

As an example, the following route matches against all paths that begin with
`/page/`:

```rust
#[get("/page/<path..>")]
fn get_page(path: PathBuf) -> T { ... }
```

The path after `/page/` will be available in the `path` parameter. The
`FromSegments` implementation for `PathBuf` ensures that `path` cannot lead to
[path traversal attacks](https://www.owasp.org/index.php/Path_Traversal). With
this, a safe and secure static file server can implemented in 4 lines:

```rust
#[get("/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}
```

## Request Guards

Sometimes we need data associated with a request that isn't a direct input.
Headers and cookies are a good example of this: they simply tag along for the
ride. Rocket makes retrieving and validating such information easy: simply add
any number of parameters to the request handler with types that implement the
[FromRequest](https://api.rocket.rs/rocket/request/trait.FromRequest.html)
trait. If the data can be retrieved from the incoming request and validated, the
handler is called. If it cannot, the handler isn't called, and the request is
forwarded or terminated. In this way, these parameters act as _guards_: they
protect the request handler from being called erroneously.

For example, to retrieve cookies and the Content-Type header from a request, we
can declare a route as follows:

```rust
#[get("/")]
fn index(cookies: &Cookies, content: ContentType) -> String { ... }
```

The [cookies example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/cookies)
illustrates how to use the `Cookies` type to get and set cookies.

You can implement `FromRequest` for your own types. For instance, to protect a
`sensitive` route from running unless an `APIKey` is present in the request
headers, you might create an `APIKey` type that implements `FromRequest` and use
it as a request guard:

```rust
#[get("/sensitive")]
fn sensitive(key: APIKey) -> &'static str { ... }
```

You might also implement `FromRequest` for an `AdminUser` type that validates
that the cookies in the incoming request authenticate an administrator. Then,
any handler with an `AdminUser` or `APIKey` type in its argument list is assured
to only be invoked if the appropriate conditions are met. Request guards
centralize policies, resulting in a simpler, safer, and more secure
applications.

## Data

At some point, your web application will need to process body data, and Rocket
makes it as simple as possible. Data processing, like much of Rocket, is type
directed. To indicate that a handler expects data, annotate it with a `data =
"<param>"` parameter, where `param` is an argument in the handler. The
argument's type must implement the
[FromData](https://api.rocket.rs/rocket/data/trait.FromData.html) trait. It
looks like this, where `T: FromData`:

```rust
#[post("/", data = "<input>")]
fn new(input: T) -> String { ... }
```

### Forms

Forms are the most common type of data handled in web applications, and Rocket
makes handling them easy. Say your application is processing a form submission
for a new todo `Task`. The form contains two fields: `complete`, a checkbox, and
`description`, a text field. You can easily handle the form request in Rocket
as follows:

```rust
#[derive(FromForm)]
struct Task {
    complete: bool,
    description: String,
}

#[post("/todo", data = "<task>")]
fn new(task: Form<Task>) -> String { ... }
```

The `Form` type implements the `FromData` trait as long as its generic parameter
implements the
[FromForm](https://api.rocket.rs/rocket/request/trait.FromForm.html) trait. In
the example, we've derived the `FromForm` trait automatically for the `Task`
structure. `FromForm` can be derived for any structure whose fields implement
[FromFormValue](https://api.rocket.rs/rocket/request/trait.FromFormValue.html).
If a `POST /todo` request arrives, the form data will automatically be parsed
into the `Task` structure. If the data that arrives isn't of the correct
Content-Type, the request is forwarded. If the data doesn't parse or is simply
invalid, a customizable `400 Bad Request` error is returned. As before, a
forward or failure can be caught by using the `Option` and `Result` types.

Fields of forms can be easily validated via implementations of the
`FromFormValue` trait. For example, if you'd like to verify that some user is
over some age in a form, then you might define a new `AdultAge` type, use it as
a field in a form structure, and implement `FromFormValue` so that it only
validates integers over that age. If a form is submitted with a bad age,
Rocket won't call a handler requiring a valid form for that structure. You can
use `Option` or `Result` types for fields to catch parse failures.

The [forms](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/forms)
and [forms kitchen
sink](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/form_kitchen_sink)
examples on GitHub provide further illustrations.

### JSON

Handling JSON data is no harder: simply use the
[JSON](https://api.rocket.rs/rocket_contrib/struct.JSON.html) type:

```rust
#[derive(Deserialize)]
struct Task {
    description: String,
    complete: bool
}

#[post("/todo", data = "<task>")]
fn new(task: JSON<Task>) -> String { ... }
```

The only condition is that the generic type to `JSON` implements the
`Deserialize` trait. See the [JSON example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/json) for a
complete example.

### Streaming

Sometimes you just want to handle the incoming data directly. For example, you
might want to stream the incoming data out to a file. Rocket makes this as
simple as possible via the
[Data](https://api.rocket.rs/rocket/data/struct.Data.html) type:

```rust
#[post("/upload", format = "text/plain", data = "<data>")]
fn upload(data: Data) -> io::Result<Plain<String>> {
    data.stream_to_file("/tmp/upload.txt").map(|n| Plain(n.to_string()))
}
```

The route above accepts any `POST` request to the `/upload` path with
`Content-Type` `text/plain`  The incoming data is streamed out to
`tmp/upload.txt` file, and the number of bytes written is returned as a plain
text response if the upload succeeds. If the upload fails, an error response is
returned. The handler above is complete. It really is that simple! See the
[GitHub example
code](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/raw_upload)
for the full crate.

## Query Strings

Query strings are handled similarly to `POST` forms. A query string can be
parsed into any structure that implements the `FromForm` trait. They are matched
against by appending a `?` followed by a dynamic parameter `<param>` to the
path.

For instance, say you change your mind and decide to use query strings instead
of `POST` forms for new todo tasks in the previous forms example, reproduced
below:

```rust
#[derive(FromForm)]
struct Task { .. }

#[post("/todo", data = "<task>")]
fn new(task: Form<Task>) -> String { ... }
```

Rocket makes the transition simple: simply declare `<task>` as a query parameter
as follows:

```rust
#[get("/todo?<task>")]
fn new(task: Task) -> String { ... }
```

Rocket will parse the query string into the `Task` structure automatically by
matching the structure field names to the query parameters. If the parse fails,
the request is forwarded to the next matching route. To catch parse failures,
you can use `Option` or `Result` as the type of the field to catch errors for.

See [the GitHub
example](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/query_params)
for a complete illustration.

## Error Catchers

When Rocket wants to return an error page to the client, Rocket invokes the
_catcher_ for that error. A catcher is like a route, except it only handles
errors. Catchers are declared via the `error` attribute, which takes a single
integer corresponding to the HTTP status code to catch. For instance, to declare
a catcher for **404** errors, you'd write:

```rust
#[error(404)]
fn not_found(req: &Request) -> String { }
```

As with routes, Rocket needs to know about a catcher before it is used to handle
errors. The process is similar to mounting: call the `catch` method with a list
of catchers via the `errors!` macro. The invocation to add the **404** catcher
declared above looks like this:

```rust
rocket::ignite().catch(errors![not_found])
```

Unlike request handlers, error handlers can only take 0, 1, or 2 parameters of
types [Request](https://api.rocket.rs/rocket/struct.Request.html) and/or
[Error](https://api.rocket.rs/rocket/enum.Error.html). At present, the `Error`
type is not particularly useful, and so it is often omitted. The
[error catcher
example](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/errors) on
GitHub illustrates their use in full.

Rocket has a default catcher for all of the standard HTTP error codes including
**404**, **500**, and more.
