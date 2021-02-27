# Requests

Together, a [`route`] attribute and function signature specify what must be true
about a request in order for the route's handler to be called. You've already
seen an example of this in action:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/world")]
fn handler() { /* .. */ }
```

This route indicates that it only matches against `GET` requests to the `/world`
route. Rocket ensures that this is the case before `handler` is called. Of
course, you can do much more than specify the method and path of a request.
Among other things, you can ask Rocket to automatically validate:

  * The type of a dynamic path segment.
  * The type of _several_ dynamic path segments.
  * The type of incoming body data.
  * The types of query strings, forms, and form values.
  * The expected incoming or outgoing format of a request.
  * Any arbitrary, user-defined security or validation policies.

The route attribute and function signature work in tandem to describe these
validations. Rocket's code generation takes care of actually validating the
properties. This section describes how to ask Rocket to validate against all of
these properties and more.

[`route`]: @api/rocket/attr.route.html

## Methods

A Rocket route attribute can be any one of `get`, `put`, `post`, `delete`,
`head`, `patch`, or `options`, each corresponding to the HTTP method to match
against. For example, the following attribute will match against `POST` requests
to the root path:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[post("/")]
# fn handler() {}
```

The grammar for these attributes is defined formally in the [`route`] API docs.

### HEAD Requests

Rocket handles `HEAD` requests automatically when there exists a `GET` route
that would otherwise match. It does this by stripping the body from the
response, if there is one. You can also specialize the handling of a `HEAD`
request by declaring a route for it; Rocket won't interfere with `HEAD` requests
your application explicitly handles.

### Reinterpreting

Because HTML forms can only be directly submitted as `GET` or `POST` requests,
Rocket _reinterprets_ request methods under certain conditions. If a `POST`
request contains a body of `Content-Type: application/x-www-form-urlencoded` and
the form's **first** field has the name `_method` and a valid HTTP method name
as its value (such as `"PUT"`), that field's value is used as the method for the
incoming request.  This allows Rocket applications to submit non-`POST` forms.
The [todo example](@example/todo/static/index.html.tera#L47) makes use of this
feature to submit `PUT` and `DELETE` requests from a web form.

## Dynamic Paths

You can declare path segments as dynamic by using angle brackets around variable
names in a route's path. For example, if we want to say _Hello!_ to anything,
not just the world, we can declare a route like so:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::http::RawStr;

#[get("/hello/<name>")]
fn hello(name: &RawStr) -> String {
    format!("Hello, {}!", name)
}
```

If we were to mount the path at the root (`.mount("/", routes![hello])`), then
any request to a path with two non-empty segments, where the first segment is
`hello`, will be dispatched to the `hello` route. For example, if we were to
visit `/hello/John`, the application would respond with `Hello, John!`.

Any number of dynamic path segments are allowed. A path segment can be of any
type, including your own, as long as the type implements the [`FromParam`]
trait. We call these types _parameter guards_. Rocket implements `FromParam` for
many of the standard library types, as well as a few special Rocket types. For
the full list of provided implementations, see the [`FromParam` API docs].
Here's a more complete route to illustrate varied usage:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/hello/<name>/<age>/<cool>")]
fn hello(name: String, age: u8, cool: bool) -> String {
    if cool {
        format!("You're a cool {} year old, {}!", age, name)
    } else {
        format!("{}, we need to talk about your coolness.", name)
    }
}
```

[`FromParam`]: @api/rocket/request/trait.FromParam.html
[`FromParam` API docs]: @api/rocket/request/trait.FromParam.html

! note: Rocket types _raw_ strings separately from decoded strings.

  You may have noticed an unfamiliar [`RawStr`] type in the code example above.
  This is a special type, provided by Rocket, that represents an unsanitized,
  unvalidated, and undecoded raw string from an HTTP message. It exists to
  separate validated string inputs, represented by types such as `String`,
  `&str`, and `Cow<str>`, from unvalidated inputs, represented by `&RawStr`. It
  also provides helpful methods to convert the unvalidated string into a
  validated one.

  Because `&RawStr` implements [`FromParam`], it can be used as the type of a
  dynamic segment, as in the example above, where the value refers to a
  potentially undecoded string. By contrast, a `String` is guaranteed to be
  decoded. Which you should use depends on whether you want direct but
  potentially unsafe access to the string (`&RawStr`), or safe access to the
  string at the cost of an allocation (`String`).

  [`RawStr`]: @api/rocket/http/struct.RawStr.html

### Multiple Segments

You can also match against multiple segments by using `<param..>` in a route
path. The type of such parameters, known as _segments guards_, must implement
[`FromSegments`]. A segments guard must be the final component of a path: any
text after a segments guard will result in a compile-time error.

As an example, the following route matches against all paths that begin with
`/page/`:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use std::path::PathBuf;

#[get("/page/<path..>")]
fn get_page(path: PathBuf) { /* ... */ }
```

The path after `/page/` will be available in the `path` parameter. The
`FromSegments` implementation for `PathBuf` ensures that `path` cannot lead to
[path traversal attacks](https://www.owasp.org/index.php/Path_Traversal). With
this, a safe and secure static file server can be implemented in 4 lines:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use std::path::{Path, PathBuf};
use rocket::response::NamedFile;

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}
```

! tip: Rocket makes it even _easier_ to serve static files!

  If you need to serve static files from your Rocket application, consider using
  the [`StaticFiles`] custom handler from [`rocket_contrib`], which makes it as
  simple as:

  `rocket.mount("/public", StaticFiles::from("/static"))`

[`rocket_contrib`]: @api/rocket_contrib/
[`StaticFiles`]: @api/rocket_contrib/serve/struct.StaticFiles.html
[`FromSegments`]: @api/rocket/request/trait.FromSegments.html

## Forwarding

Let's take a closer look at the route attribute and signature pair from a
previous example:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/hello/<name>/<age>/<cool>")]
fn hello(name: String, age: u8, cool: bool) { /* ... */ }
```

What if `cool` isn't a `bool`? Or, what if `age` isn't a `u8`? When a parameter
type mismatch occurs, Rocket _forwards_ the request to the next matching route,
if there is any. This continues until a route doesn't forward the request or
there are no remaining routes to try. When there are no remaining routes, a
customizable **404 error** is returned.

Routes are attempted in increasing _rank_ order. Rocket chooses a default
ranking from -6 to -1, detailed in the next section, but a route's rank can also
be manually set with the `rank` attribute. To illustrate, consider the following
routes:

```rust
# #[macro_use] extern crate rocket;

# use rocket::http::RawStr;

#[get("/user/<id>")]
fn user(id: usize) { /* ... */ }

#[get("/user/<id>", rank = 2)]
fn user_int(id: isize) { /* ... */ }

#[get("/user/<id>", rank = 3)]
fn user_str(id: &RawStr) { /* ... */ }

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![user, user_int, user_str])
}
```

Notice the `rank` parameters in `user_int` and `user_str`. If we run this
application with the routes mounted at the root path, as is done in `rocket()`
above, requests to `/user/<id>` (such as `/user/123`, `/user/Bob`, and so on)
will be routed as follows:

  1. The `user` route matches first. If the string at the `<id>` position is an
     unsigned integer, then the `user` handler is called. If it is not, then the
     request is forwarded to the next matching route: `user_int`.

  2. The `user_int` route matches next. If `<id>` is a signed integer,
     `user_int` is called. Otherwise, the request is forwarded.

  3. The `user_str` route matches last. Since `<id>` is always a string, the
     route always matches. The `user_str` handler is called.

! note: A route's rank appears in **[brackets]** during launch.

  You'll also find a route's rank logged in brackets during application launch:
  `GET /user/<id> [3] (user_str)`.

Forwards can be _caught_ by using a `Result` or `Option` type. For example, if
the type of `id` in the `user` function was `Result<usize, &RawStr>`, then `user`
would never forward. An `Ok` variant would indicate that `<id>` was a valid
`usize`, while an `Err` would indicate that `<id>` was not a `usize`. The
`Err`'s value would contain the string that failed to parse as a `usize`.

! tip: It's not just forwards that can be caught!

  In general, when any guard fails for any reason, including parameter guards,
  you can use an `Option` or `Result` type in its place to catch the failure.

By the way, if you were to omit the `rank` parameter in the `user_str` or
`user_int` routes, Rocket would emit an error and abort launch, indicating that
the routes _collide_, or can match against similar incoming requests. The `rank`
parameter resolves this collision.

### Default Ranking

If a rank is not explicitly specified, Rocket assigns a default ranking. By
default, routes with static paths and query strings have lower ranks (higher
precedence) while routes with dynamic paths and without query strings have
higher ranks (lower precedence). The table below describes the default ranking
of a route given its properties.

| static path | query         | rank | example             |
|-------------|---------------|------|---------------------|
| yes         | partly static | -6   | `/hello?world=true` |
| yes         | fully dynamic | -5   | `/hello/?<world>`   |
| yes         | none          | -4   | `/hello`            |
| no          | partly static | -3   | `/<hi>?world=true`  |
| no          | fully dynamic | -2   | `/<hi>?<world>`     |
| no          | none          | -1   | `/<hi>`             |

## Query Strings

Query segments can be declared static or dynamic in much the same way as path
segments:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use rocket::http::RawStr;

#[get("/hello?wave&<name>")]
fn hello(name: &RawStr) -> String {
    format!("Hello, {}!", name.as_str())
}
```

The `hello` route above matches any `GET` request to `/hello` that has at least
one query key of `name` and a query segment of `wave` in any order, ignoring any
extra query segments. The value of the `name` query parameter is used as the
value of the `name` function argument. For instance, a request to
`/hello?wave&name=John` would return `Hello, John!`. Other requests that would
result in the same response include:

  * `/hello?name=John&wave` (reordered)
  * `/hello?name=John&wave&id=123` (extra segments)
  * `/hello?id=123&name=John&wave` (reordered, extra segments)
  * `/hello?name=Bob&name=John&wave` (last value taken)

Any number of dynamic query segments are allowed. A query segment can be of any
type, including your own, as long as the type implements the [`FromFormValue`]
trait.

[`FromFormValue`]: @api/rocket/request/trait.FromFormValue.html

### Optional Parameters

Query parameters are allowed to be _missing_. As long as a request's query
string contains all of the static components of a route's query string, the
request will be routed to that route. This allows for optional parameters,
validating even when a parameter is missing.

To achieve this, use `Option<T>` as the parameter type. Whenever the query
parameter is missing in a request, `None` will be provided as the value.  A
route using `Option<T>` looks as follows:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/hello?wave&<name>")]
fn hello(name: Option<String>) -> String {
    name.map(|name| format!("Hi, {}!", name))
        .unwrap_or_else(|| "Hello!".into())
}
```

Any `GET` request with a path of `/hello` and a `wave` query segment will be
routed to this route. If a `name=value` query segment is present, the route
returns the string `"Hi, value!"`. If no `name` query segment is present, the
route returns `"Hello!"`.

Just like a parameter of type `Option<T>` will have the value `None` if the
parameter is missing from a query, a parameter of type `bool` will have the
value `false` if it is missing. The default value for a missing parameter can be
customized for your own types that implement `FromFormValue` by implementing
[`FromFormValue::default()`].

[`FromFormValue::default()`]: @api/rocket/request/trait.FromFormValue.html#method.default

### Multiple Segments

As with paths, you can also match against multiple segments in a query by using
`<param..>`. The type of such parameters, known as _query guards_, must
implement the [`FromQuery`] trait. Query guards must be the final component of a
query: any text after a query parameter will result in a compile-time error.

A query guard validates all otherwise unmatched (by static or dynamic query
parameters) query segments. While you can implement [`FromQuery`] yourself, most
use cases will be handled by using the [`Form`] or [`LenientForm`] query guard.
The [Forms](#forms) section explains using these types in detail. In short,
these types allow you to use a structure with named fields to automatically
validate query/form parameters:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::request::Form;

#[derive(FromForm)]
struct User {
    name: String,
    account: usize,
}

#[get("/item?<id>&<user..>")]
fn item(id: usize, user: Form<User>) { /* ... */ }
```

For a request to `/item?id=100&name=sandal&account=400`, the `item` route above
sets `id` to `100` and `user` to `User { name: "sandal", account: 400 }`. To
catch forms that fail to validate, use a type of `Option` or `Result`:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use rocket::request::Form;
# #[derive(FromForm)] struct User { name: String, account: usize, }

#[get("/item?<id>&<user..>")]
fn item(id: usize, user: Option<Form<User>>) { /* ... */ }
```

For more query handling examples, see [the `query_params`
example](@example/query_params).

[`FromQuery`]: @api/rocket/request/trait.FromQuery.html

## Request Guards

Request guards are one of Rocket's most powerful instruments. As the name might
imply, a request guard protects a handler from being called erroneously based on
information contained in an incoming request. More specifically, a request guard
is a type that represents an arbitrary validation policy. The validation policy
is implemented through the [`FromRequest`] trait. Every type that implements
`FromRequest` is a request guard.

Request guards appear as inputs to handlers. An arbitrary number of request
guards can appear as arguments in a route handler. Rocket will automatically
invoke the [`FromRequest`] implementation for request guards before calling the
handler. Rocket only dispatches requests to a handler when all of its guards
pass.

For instance, the following dummy handler makes use of three request guards,
`A`, `B`, and `C`. An input can be identified as a request guard if it is not
named in the route attribute.

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# type A = rocket::http::Method;
# type B = A;
# type C = A;

#[get("/<param>")]
fn index(param: isize, a: A, b: B, c: C) { /* ... */ }
```

Request guards always fire in left-to-right declaration order. In the example
above, the order will be `A` followed by `B` followed by `C`. Failure is
short-circuiting; if one guard fails, the remaining are not attempted. To learn
more about request guards and implementing them, see the [`FromRequest`]
documentation.

[`FromRequest`]: @api/rocket/request/trait.FromRequest.html
[`CookieJar`]: @api/rocket/http/struct.CookieJar.html

### Custom Guards

You can implement `FromRequest` for your own types. For instance, to protect a
`sensitive` route from running unless an `ApiKey` is present in the request
headers, you might create an `ApiKey` type that implements `FromRequest` and
then use it as a request guard:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}
# type ApiKey = rocket::http::Method;

#[get("/sensitive")]
fn sensitive(key: ApiKey) { /* .. */ }
```

You might also implement `FromRequest` for an `AdminUser` type that
authenticates an administrator using incoming cookies. Then, any handler with an
`AdminUser` or `ApiKey` type in its argument list is assured to only be invoked
if the appropriate conditions are met. Request guards centralize policies,
resulting in a simpler, safer, and more secure applications.

### Guard Transparency

When a request guard type can only be created through its [`FromRequest`]
implementation, and the type is not `Copy`, the existence of a request guard
value provides a _type-level proof_ that the current request has been validated
against an arbitrary policy. This provides powerful means of protecting your
application against access-control violations by requiring data accessing
methods to _witness_ a proof of authorization via a request guard. We call the
notion of using a request guard as a witness _guard transparency_.

As a concrete example, the following application has a function,
`health_records`, that returns all of the health records in a database. Because
health records are sensitive information, they should only be accessible by
super users. The `SuperUser` request guard authenticates and authorizes a super
user, and its `FromRequest` implementation is the only means by which a
`SuperUser` can be constructed. By declaring the `health_records` function as
follows, access control violations against health records are guaranteed to be
prevented at _compile-time_:

```rust
# type Records = ();
# type SuperUser = ();
fn health_records(user: &SuperUser) -> Records { /* ... */ }
```

The reasoning is as follows:

  1. The `health_records` function requires an `&SuperUser` type.
  2. The only constructor for a `SuperUser` type is `FromRequest`.
  3. Only Rocket can provide an active `&Request` to construct via `FromRequest`.
  4. Thus, there must be a `Request` authorizing a `SuperUser` to call
     `health_records`.

! note

  At the expense of a lifetime parameter in the guard type, guarantees can be
  made even stronger by tying the lifetime of the `Request` passed to
  `FromRequest` to the request guard, ensuring that the guard value always
  corresponds to an _active_ request.

We recommend leveraging request guard transparency for _all_ data accesses.

### Forwarding Guards

Request guards and forwarding are a powerful combination for enforcing policies.
To illustrate, we consider how a simple authorization system might be
implemented using these mechanisms.

We start with two request guards:

  * `User`: A regular, authenticated user.

    The `FromRequest` implementation for `User` checks that a cookie identifies
    a user and returns a `User` value if so. If no user can be authenticated,
    the guard forwards.

  * `AdminUser`: A user authenticated as an administrator.

    The `FromRequest` implementation for `AdminUser` checks that a cookie
    identifies an _administrative_ user and returns an `AdminUser` value if so.
    If no user can be authenticated, the guard forwards.

We now use these two guards in combination with forwarding to implement the
following three routes, each leading to an administrative control panel at
`/admin`:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# type Template = ();
# type AdminUser = rocket::http::Method;
# type User = rocket::http::Method;

use rocket::response::Redirect;

#[get("/login")]
fn login() -> Template { /* .. */ }

#[get("/admin")]
fn admin_panel(admin: AdminUser) -> &'static str {
    "Hello, administrator. This is the admin panel!"
}

#[get("/admin", rank = 2)]
fn admin_panel_user(user: User) -> &'static str {
    "Sorry, you must be an administrator to access this page."
}

#[get("/admin", rank = 3)]
fn admin_panel_redirect() -> Redirect {
    Redirect::to(uri!(login))
}
```

The three routes above encode authentication _and_ authorization. The
`admin_panel` route only succeeds if an administrator is logged in. Only then is
the admin panel displayed. If the user is not an admin, the `AdminUser` guard
will forward. Since the `admin_panel_user` route is ranked next highest, it is
attempted next. This route succeeds if there is _any_ user signed in, and an
authorization failure message is displayed. Finally, if a user isn't signed in,
the `admin_panel_redirect` route is attempted. Since this route has no guards,
it always succeeds. The user is redirected to a log in page.

## Cookies

A reference to a [`CookieJar`] is an important, built-in request guard: it
allows you to get, set, and remove cookies. Because `&CookieJar` is a request
guard, an argument of its type can simply be added to a handler:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}
use rocket::http::CookieJar;

#[get("/")]
fn index(cookies: &CookieJar<'_>) -> Option<String> {
    cookies.get("message").map(|crumb| format!("Message: {}", crumb.value()))
}
```

This results in the incoming request's cookies being accessible from the
handler. The example above retrieves a cookie named `message`. Cookies can also
be set and removed using the `CookieJar` guard. The [cookies example] on GitHub
illustrates further use of the `CookieJar` type to get and set cookies, while
the [`CookieJar`] documentation contains complete usage information.

[cookies example]: @example/cookies

### Private Cookies

Cookies added via the [`CookieJar::add()`] method are set _in the clear._ In
other words, the value set is visible to the client. For sensitive data, Rocket
provides _private_ cookies. Private cookies are similar to regular cookies
except that they are encrypted using authenticated encryption, a form of
encryption which simultaneously provides confidentiality, integrity, and
authenticity. Thus, private cookies cannot be inspected, tampered with, or
manufactured by clients. If you prefer, you can think of private cookies as
being signed and encrypted.

Support for private cookies must be manually enabled via the `secrets` crate
feature:

```toml
## in Cargo.toml
rocket = { version = "0.5.0-dev", features = ["secrets"] }
```

The API for retrieving, adding, and removing private cookies is identical except
methods are suffixed with `_private`. These methods are: [`get_private`],
[`get_private_pending`], [`add_private`], and [`remove_private`]. An example of
their usage is below:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::http::{Cookie, CookieJar};
use rocket::response::{Flash, Redirect};

/// Retrieve the user's ID, if any.
#[get("/user_id")]
fn user_id(cookies: &CookieJar<'_>) -> Option<String> {
    cookies.get_private("user_id")
        .map(|crumb| format!("User ID: {}", crumb.value()))
}

/// Remove the `user_id` cookie.
#[post("/logout")]
fn logout(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named("user_id"));
    Flash::success(Redirect::to("/"), "Successfully logged out.")
}
```

[`CookieJar::add()`]: @api/rocket/http/struct.CookieJar.html#method.add

### Secret Key

To encrypt private cookies, Rocket uses the 256-bit key specified in the
`secret_key` configuration parameter. When compiled in debug mode, a fresh key
is generated automatically. In release mode, Rocket requires you to set a secret
key if the `secrets` feature is enabled. Failure to do so results in a hard
error at launch time. The value of the parameter may either be a 256-bit base64
or hex string or a 32-byte slice.

Generating a string suitable for use as a `secret_key` configuration value is
usually done through tools like `openssl`. Using `openssl`, a 256-bit base64 key
can be generated with the command `openssl rand -base64 32`.

For more information on configuration, see the [Configuration](../configuration)
section of the guide.

[`get_private`]: @api/rocket/http/struct.CookieJar.html#method.get_private
[`get_private_pending`]: @api/rocket/http/struct.CookieJar.html#method.get_private_pending
[`add_private`]: @api/rocket/http/struct.CookieJar.html#method.add_private
[`remove_private`]: @api/rocket/http/struct.CookieJar.html#method.remove_private

## Format

A route can specify the data format it is willing to accept or respond with by
using the `format` route parameter. The value of the parameter is a string
identifying an HTTP media type or a shorthand variant. For instance, for JSON
data, the string `application/json` or simply `json` can be used.

When a route indicates a payload-supporting method (`PUT`, `POST`, `DELETE`, and
`PATCH`), the `format` route parameter instructs Rocket to check against the
`Content-Type` header of the incoming request. Only requests where the
`Content-Type` header matches the `format` parameter will match to the route.

As an example, consider the following route:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# type User = rocket::data::Data;

#[post("/user", format = "application/json", data = "<user>")]
fn new_user(user: User) { /* ... */ }
```

The `format` parameter in the `post` attribute declares that only incoming
requests with `Content-Type: application/json` will match `new_user`. (The
`data` parameter is described in the next section.) Shorthand is also supported
for the most common `format` arguments. Instead of using the full Content-Type,
`format = "application/json"`, you can also write shorthands like `format =
"json"`. For a full list of available shorthands, see the
[`ContentType::parse_flexible()`] documentation.

When a route indicates a non-payload-supporting method (`GET`, `HEAD`,
`OPTIONS`) the `format` route parameter instructs Rocket to check against the
`Accept` header of the incoming request. Only requests where the preferred media
type in the `Accept` header matches the `format` parameter will match to the
route.

As an example, consider the following route:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}
# type User = ();

#[get("/user/<id>", format = "json")]
fn user(id: usize) -> User { /* .. */ }
```

The `format` parameter in the `get` attribute declares that only incoming
requests with `application/json` as the preferred media type in the `Accept`
header will match `user`. If instead the route had been declared as `post`,
Rocket would match the `format` against the `Content-Type` header of the
incoming response.

[`ContentType::parse_flexible()`]: @api/rocket/http/struct.ContentType.html#method.parse_flexible

## Body Data

Body data processing, like much of Rocket, is type directed. To indicate that a
handler expects body data, annotate it with `data = "<param>"`, where `param` is
an argument in the handler. The argument's type must implement the [`FromData`]
trait. It looks like this, where `T` is assumed to implement `FromData`:

```rust
# #[macro_use] extern crate rocket;

# type T = rocket::data::Data;

#[post("/", data = "<input>")]
fn new(input: T) { /* .. */ }
```

Any type that implements [`FromData`] is also known as _a data guard_.

[`FromData`]: @api/rocket/data/trait.FromData.html

### Forms

Forms are one of the most common types of data handled in web applications, and
Rocket makes handling them easy. Say your application is processing a form
submission for a new todo `Task`. The form contains two fields: `complete`, a
checkbox, and `description`, a text field. You can easily handle the form
request in Rocket as follows:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::request::Form;

#[derive(FromForm)]
struct Task {
    complete: bool,
    description: String,
}

#[post("/todo", data = "<task>")]
fn new(task: Form<Task>) { /* .. */ }
```

The [`Form`] type implements the `FromData` trait as long as its generic
parameter implements the [`FromForm`] trait. In the example, we've derived the
`FromForm` trait automatically for the `Task` structure. `FromForm` can be
derived for any structure whose fields implement [`FromFormValue`]. If a `POST
/todo` request arrives, the form data will automatically be parsed into the
`Task` structure. If the data that arrives isn't of the correct Content-Type,
the request is forwarded. If the data doesn't parse or is simply invalid, a
customizable `400 - Bad Request` or `422 - Unprocessable Entity` error is
returned. As before, a forward or failure can be caught by using the `Option`
and `Result` types:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use rocket::request::Form;
# #[derive(FromForm)] struct Task { complete: bool, description: String, }

#[post("/todo", data = "<task>")]
fn new(task: Option<Form<Task>>) { /* .. */ }
```

[`Form`]: @api/rocket/request/struct.Form.html
[`FromForm`]: @api/rocket/request/trait.FromForm.html
[`FromFormValue`]: @api/rocket/request/trait.FromFormValue.html

#### Lenient Parsing

Rocket's `FromForm` parsing is _strict_ by default. In other words, a `Form<T>`
will parse successfully from an incoming form only if the form contains the
exact set of fields in `T`. Said another way, a `Form<T>` will error on missing
and/or extra fields. For instance, if an incoming form contains the fields "a",
"b", and "c" while `T` only contains "a" and "c", the form _will not_ parse as
`Form<T>`.

Rocket allows you to opt-out of this behavior via the [`LenientForm`] data type.
A `LenientForm<T>` will parse successfully from an incoming form as long as the
form contains a superset of the fields in `T`. Said another way, a
`LenientForm<T>` automatically discards extra fields without error. For
instance, if an incoming form contains the fields "a", "b", and "c" while `T`
only contains "a" and "c", the form _will_ parse as `LenientForm<T>`.

You can use a `LenientForm` anywhere you'd use a `Form`. Its generic parameter
is also required to implement `FromForm`. For instance, we can simply replace
`Form` with `LenientForm` above to get lenient parsing:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::request::LenientForm;

#[derive(FromForm)]
struct Task {
    /* .. */
    # complete: bool,
    # description: String,
}

#[post("/todo", data = "<task>")]
fn new(task: LenientForm<Task>) { /* .. */ }
```

[`LenientForm`]: @api/rocket/request/struct.LenientForm.html

#### Field Renaming

By default, Rocket matches the name of an incoming form field to the name of a
structure field. While this behavior is typical, it may also be desired to use
different names for form fields and struct fields while still parsing as
expected. You can ask Rocket to look for a different form field for a given
structure field by using the `#[form(field = "name")]` field annotation.

As an example, say that you're writing an application that receives data from an
external service. The external service `POST`s a form with a field named `type`.
Since `type` is a reserved keyword in Rust, it cannot be used as the name of a
field. To get around this, you can use field renaming as follows:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[derive(FromForm)]
struct External {
    #[form(field = "type")]
    api_type: String
}
```

Rocket will then match the form field named `type` to the structure field named
`api_type` automatically.

#### Field Validation

Fields of forms can be easily validated via implementations of the
[`FromFormValue`] trait. For example, if you'd like to verify that some user is
over some age in a form, then you might define a new `AdultAge` type, use it as
a field in a form structure, and implement `FromFormValue` so that it only
validates integers over that age:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::http::RawStr;
use rocket::request::FromFormValue;

struct AdultAge(usize);

impl<'v> FromFormValue<'v> for AdultAge {
    type Error = &'v RawStr;

    fn from_form_value(form_value: &'v RawStr) -> Result<AdultAge, &'v RawStr> {
        match form_value.parse::<usize>() {
            Ok(age) if age >= 21 => Ok(AdultAge(age)),
            _ => Err(form_value),
        }
    }
}

#[derive(FromForm)]
struct Person {
    age: AdultAge
}
```

If a form is submitted with a bad age, Rocket won't call a handler requiring a
valid form for that structure. You can use `Option` or `Result` types for fields
to catch parse failures:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# type AdultAge = usize;

#[derive(FromForm)]
struct Person {
    age: Option<AdultAge>
}
```

The `FromFormValue` trait can also be derived for enums with nullary fields:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[derive(FromFormValue)]
enum MyValue {
    First,
    Second,
    Third,
}
```

The derive generates an implementation of the `FromFormValue` trait for the
decorated enum. The implementation returns successfully when the form value
matches, case insensitively, the stringified version of a variant's name,
returning an instance of said variant.

The [form validation](@example/form_validation) and [form kitchen
sink](@example/form_kitchen_sink) examples provide further illustrations.

### JSON

Handling JSON data is no harder: simply use the
[`Json`](@api/rocket_contrib/json/struct.Json.html) type from
[`rocket_contrib`]:

```rust
# #[macro_use] extern crate rocket;
# extern crate rocket_contrib;
# fn main() {}

use serde::Deserialize;
use rocket_contrib::json::Json;

#[derive(Deserialize)]
struct Task {
    description: String,
    complete: bool
}

#[post("/todo", data = "<task>")]
fn new(task: Json<Task>) { /* .. */ }
```

The only condition is that the generic type in `Json` implements the
`Deserialize` trait from [Serde](https://github.com/serde-rs/json). See the
[JSON example] on GitHub for a complete example.

[JSON example]: @example/json

### Streaming

Sometimes you just want to handle incoming data directly. For example, you might
want to stream the incoming data out to a file. Rocket makes this as simple as
possible via the [`Data`](@api/rocket/data/struct.Data.html) type:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::data::{Data, ToByteUnit};
use rocket::response::Debug;

#[post("/upload", format = "plain", data = "<data>")]
async fn upload(data: Data) -> Result<String, Debug<std::io::Error>> {
    let bytes_written = data.open(128.kibibytes())
        .stream_to_file("/tmp/upload.txt")
        .await?;

    Ok(bytes_written.to_string())
}
```

The route above accepts any `POST` request to the `/upload` path with
`Content-Type: text/plain`  At most 128KiB (`128 << 10` bytes) of the incoming
data are streamed out to `tmp/upload.txt`, and the number of bytes written is
returned as a plain text response if the upload succeeds. If the upload fails,
an error response is returned. The handler above is complete. It really is that
simple! See the [GitHub example code](@example/raw_upload) for the full crate.

! note: Rocket requires setting limits when reading incoming data.

  To aid in preventing DoS attacks, Rocket requires you to specify, as a
  [`ByteUnit`](@api/rocket/data/struct.ByteUnit.html), the amount of data you're
  willing to accept from the client when `open`ing a data stream. The
  [`ToByteUnit`](@api/rocket/data/trait.ToByteUnit.html) trait makes specifying
  such a value as idiomatic as `128.kibibytes()`.

## Async Routes

Rocket makes it easy to use `async/await` in routes.

```rust
# #[macro_use] extern crate rocket;
use rocket::tokio::time::{sleep, Duration};
#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}
```

First, notice that the route function is an `async fn`. This enables
the use of `await` inside the handler. `sleep` is an asynchronous
function, so we must `await` it.

## Error Catchers

Application processing is fallible. Errors arise from the following sources:

  * A failing guard.
  * A failing responder.
  * A routing failure.

If any of these occur, Rocket returns an error to the client. To generate the
error, Rocket invokes the _catcher_ corresponding to the error's status code.
Catchers are similar to routes except in that:

  1. Catchers are only invoked on error conditions.
  2. Catchers are declared with the `catch` attribute.
  3. Catchers are _registered_ with [`register()`] instead of [`mount()`].
  4. Any modifications to cookies are cleared before a catcher is invoked.
  5. Error catchers cannot invoke guards.
  6. Error catchers should not fail to produce a response.

To declare a catcher for a given status code, use the [`catch`] attribute, which
takes a single integer corresponding to the HTTP status code to catch. For
instance, to declare a catcher for `404 Not Found` errors, you'd write:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::Request;

#[catch(404)]
fn not_found(req: &Request) { /* .. */ }
```

Catchers may take zero, one, or two arguments. If the catcher takes one
argument, it must be of type [`&Request`]. It it takes two, they must be of type
[`Status`] and [`&Request`], in that order. As with routes, the return type must
implement `Responder`. A concrete implementation may look like:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use rocket::Request;

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}
```

Also as with routes, Rocket needs to know about a catcher before it is used to
handle errors. The process, known as "registering" a catcher, is similar to
mounting a route: call the [`register()`] method with a list of catchers via the
[`catchers!`] macro. The invocation to add the **404** catcher declared above
looks like:

```rust
# #[macro_use] extern crate rocket;

# use rocket::Request;
# #[catch(404)] fn not_found(req: &Request) { /* .. */ }

fn main() {
    rocket::ignite().register(catchers![not_found]);
}
```

### Default Catchers

If no catcher for a given status code has been registered, Rocket calls the
_default_ catcher. Rocket provides a default catcher for all applications
automatically, so providing one is usually unnecessary. Rocket's built-in
default catcher can handle all errors. It produces HTML or JSON, depending on
the value of the `Accept` header. As such, a default catcher, or catchers in
general, only need to be registered if an error needs to be handled in a custom
fashion.

Declaring a default catcher is done with `#[catch(default)]`:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::Request;
use rocket::http::Status;

#[catch(default)]
fn default_catcher(status: Status, request: &Request) { /* .. */ }
```

It must similarly be registered with [`register()`].

The [error catcher example](@example/errors) illustrates their use in full,
while the [`Catcher`] API documentation provides further details.

[`catch`]: @api/rocket/attr.catch.html
[`register()`]: @api/rocket/struct.Rocket.html#method.register
[`mount()`]: @api/rocket/struct.Rocket.html#method.mount
[`catchers!`]: @api/rocket/macro.catchers.html
[`&Request`]: @api/rocket/struct.Request.html
[`Status`]: @api/rocket/http/struct.Status.html
[`Catcher`]: @api/rocket/catcher/struct.Catcher.html
