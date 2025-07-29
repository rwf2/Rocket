+++
summary = "handling request and body data: control-flow, parsing, validation"
+++

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

[`route`]: @api/master/rocket/attr.route.html

## Methods

A Rocket route attribute can either be method-specific, any one of `get`, `put`,
`post`, `delete`, `head`, `patch`, or `options`, or the generic [`route`], which
allows explicitly specifying any valid HTTP [`Method`] or no method at all, to
match again _any_ method. Consider the following examples:

  * Match a `POST` request to `/`:

    ```rust
    # use rocket::post;
    #[post("/")]
    # fn handler() {}
    ```

  * Match a `PATCH` request to `/fix`:

    ```rust
    # use rocket::patch;
    #[patch("/fix")]
    # fn handler() {}
    ```

  * Match a `PROPFIND` request to `/collection`:

    ```rust
    # use rocket::route;
    #[route("/collection", method = PROPFIND)]
    # fn handler() {}
    ```

  * Match a `VERSION-CONTROL` request to `/collection`:

    ```rust
    # use rocket::route;
    #[route("/resource", method = "VERSION-CONTROL")]
    # fn handler() {}
    ```

  * Match a request to `/page` with _any_ method:

    ```rust
    # use rocket::route;
    #[route("/page")]
    # fn handler() {}
    ```

The grammar for these attributes is defined formally in the [`route`] API docs.

[`Method`]: @api/master/rocket/http/enum.Method.html

### HEAD Requests

Rocket handles `HEAD` requests automatically when there exists a `GET` route
that would otherwise match. It does this by stripping the body from the
response, if there is one. You can also specialize the handling of a `HEAD`
request by declaring a route for it; Rocket won't interfere with `HEAD` requests
your application explicitly handles.

### Reinterpreting

Because web browsers only support submitting HTML forms as `GET` or `POST` requests,
Rocket _reinterprets_ request methods under certain conditions. If a `POST`
request contains a body of `Content-Type: application/x-www-form-urlencoded` and
the form's **first** field has the name `_method` and a valid HTTP method name
as its value (such as `"PUT"`), that field's value is used as the method for the
incoming request.  This allows Rocket applications to submit non-`POST` forms.
The [todo example](@git/master/examples/todo/static/index.html.tera#L47) makes use of this
feature to submit `PUT` and `DELETE` requests from a web form.

## Dynamic Paths

You can declare path segments as dynamic by using angle brackets around variable
names in a route's path. For example, if we want to say _Hello!_ to anything,
not just the world, we can declare a route like so:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/hello/<name>")]
fn hello(name: &str) -> String {
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
fn hello(name: &str, age: u8, cool: bool) -> String {
    if cool {
        format!("You're a cool {} year old, {}!", age, name)
    } else {
        format!("{}, we need to talk about your coolness.", name)
    }
}
```

[`FromParam`]: @api/master/rocket/request/trait.FromParam.html
[`FromParam` API docs]: @api/master/rocket/request/trait.FromParam.html

### Multiple Segments

You can also match against multiple segments by using `<param..>` in a route
path. The type of such parameters, known as _segments guards_, must implement
[`FromSegments`]. A segments guard must be the final component of a path: any
text after a segments guard will result in a compile-time error.

As an example, the following route matches against all paths that begin with
`/page`:

```rust
# use rocket::get;
use std::path::PathBuf;

#[get("/page/<path..>")]
fn get_page(path: PathBuf) { /* ... */ }
```

The path after `/page/` will be available in the `path` parameter, which may be
empty for paths that are simply `/page`, `/page/`, `/page//`, and so on. The
`FromSegments` implementation for `PathBuf` ensures that `path` cannot lead to
[path traversal attacks]. With this, a safe and secure static file server can be
implemented in just 4 lines:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use std::path::{Path, PathBuf};
use rocket::fs::NamedFile;

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}
```

[path traversal attacks]: https://owasp.org/www-community/attacks/Path_Traversal

! tip: Rocket makes it even _easier_ to serve static files!

  If you need to serve static files from your Rocket application, consider using
  [`FileServer`], which makes it as simple as:

  ```rust
  # #[macro_use] extern crate rocket;

  use rocket::fs::FileServer;

  #[launch]
  fn rocket() -> _ {
      rocket::build()
           // serve files from `/www/static` at path `/public`
          .mount("/public", FileServer::new("/www/static"))
  }
  ```

[`FileServer`]: @api/master/rocket/fs/struct.FileServer.html
[`FromSegments`]: @api/master/rocket/request/trait.FromSegments.html

### Ignored Segments

A component of a route can be fully ignored by using `<_>`, and multiple
components can be ignored by using `<_..>`. In other words, the wildcard name
`_` is a dynamic parameter name that ignores that dynamic parameter. An ignored
parameter must not appear in the function argument list. A segment declared as
`<_>` matches anything in a single segment while segments declared as `<_..>`
match any number of segments with no conditions.

As an example, the `foo_bar` route below matches any `GET` request with a
3-segment URI that starts with `/foo/` and ends with `/bar`. The `everything`
route below matches _every_ GET request.

```rust
# #[macro_use] extern crate rocket;

#[get("/foo/<_>/bar")]
fn foo_bar() -> &'static str {
    "Foo _____ bar!"
}

#[get("/<_..>")]
fn everything() -> &'static str {
    "Hey, you're here."
}

# // Ensure there are no collisions.
# rocket_docs_tests::client(routes![foo_bar, everything]);
```

## Forwarding

Let's take a closer look at this route attribute and signature pair from a
previous example:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

#[get("/hello/<name>/<age>/<cool>")]
fn hello(name: &str, age: u8, cool: bool) { /* ... */ }
```

What if `cool` isn't a `bool`? Or, what if `age` isn't a `u8`? When a parameter
type mismatch occurs, Rocket _forwards_ the request to the next matching route,
if there is any. This continues until a route succeeds or fails, or there are no
other matching routes to try. When there are no remaining routes, the [error
catcher](#error-catchers) associated with the status set by the last forwarding
guard is called.

Routes are attempted in increasing _rank_ order. Every route has an associated
rank. If one is not specified, Rocket chooses a default rank designed to avoid
collisions based on a route's _color_, detailed in the [next
section](#default-ranking), but a route's rank can also be manually set with the
`rank` attribute. To illustrate, consider the following routes:

```rust
# #[macro_use] extern crate rocket;

#[get("/user/<id>")]
fn user(id: usize) { /* ... */ }

#[get("/user/<id>", rank = 2)]
fn user_int(id: isize) { /* ... */ }

#[get("/user/<id>", rank = 3)]
fn user_str(id: &str) { /* ... */ }

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![user, user_int, user_str])
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
the type of `id` in the `user` function was `Result<usize, &str>`, then `user`
would never forward. An `Ok` variant would indicate that `<id>` was a valid
`usize`, while an `Err` would indicate that `<id>` was not a `usize`. The
`Err`'s value would contain the string that failed to parse as a `usize`.

! tip: It's not just forwards that can be caught!

  In general, when any guard fails for any reason, including parameter guards,
  you can use an `Option` or `Result` type in its place to catch the error.

By the way, if you were to omit the `rank` parameter in the `user_str` or
`user_int` routes, Rocket would emit an error and abort launch, indicating that
the routes _collide_, or can match against similar incoming requests. The `rank`
parameter resolves this collision.

### Default Ranking

If a rank is not explicitly specified, Rocket assigns a default rank to a route.
Default ranks range from `-12` to `-1`. This differs from ranks that can be
assigned manually in a route attribute, which must be positive numbers. The
default rank prefers static segments over dynamic segments in both paths and
queries: the _more_ static a route's path and query are, the higher its
precedence.

There are three "colors" assignable to each path and query:

  1. `static`, meaning all components are static
  2. `partial`, meaning at least one component is dynamic
  3. `wild`, meaning all components are dynamic

Additionally, a query may have _no_ color (`none`) if there is no query.

Static paths carry more weight than static queries. The same is true for partial
and wild paths. This results in the following default ranking table:

| path color | query color | default rank |
|------------|-------------|--------------|
| static     | static      | -12          |
| static     | partial     | -11          |
| static     | wild        | -10          |
| static     | none        | -9           |
| partial    | static      | -8           |
| partial    | partial     | -7           |
| partial    | wild        | -6           |
| partial    | none        | -5           |
| wild       | static      | -4           |
| wild       | partial     | -3           |
| wild       | wild        | -2           |
| wild       | none        | -1           |

Recall that _lower_ ranks have _higher_ precedence. As an example, consider this
application from before:

```rust
# #[macro_use] extern crate rocket;

#[get("/foo/<_>/bar")]
fn foo_bar() { }

#[get("/<_..>")]
fn everything() { }
#
# // Ensure there are no collisions.
# rocket_docs_tests::client(routes![foo_bar, everything]);
```

Default ranking ensures that `foo_bar`, with a "partial" path color, has higher
precedence than `everything` with a "wild" path color. This default ranking
prevents what would have otherwise been a routing collision.

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
above, the order will be `A` followed by `B` followed by `C`. Errors are
short-circuiting; if one guard fails, the remaining are not attempted. To learn
more about request guards and implementing them, see the [`FromRequest`]
documentation.

[`FromRequest`]: @api/master/rocket/request/trait.FromRequest.html
[`CookieJar`]: @api/master/rocket/http/struct.CookieJar.html

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
    the guard forwards with a 401 Unauthorized status.

  * `AdminUser`: A user authenticated as an administrator.

    The `FromRequest` implementation for `AdminUser` checks that a cookie
    identifies an _administrative_ user and returns an `AdminUser` value if so.
    If no user can be authenticated, the guard forwards with a 401 Unauthorized
    status.

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
authorization error message is displayed. Finally, if a user isn't signed in,
the `admin_panel_redirect` route is attempted. Since this route has no guards,
it always succeeds. The user is redirected to a log in page.

### Fallible Guards

A failing or forwarding guard can be "caught" in handler, preventing it from
failing or forwarding, via the `Option<T>` and `Result<T, E>` guards. When a
guard `T` fails or forwards, `Option<T>` will be `None`. If a guard `T` fails
with error `E`, `Result<T, E>` will be `Err(E)`.

As an example, for the `User` guard above, instead of allowing the guard to
forward in `admin_panel_user`, we might want to detect it and handle it
dynamically:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# type Template = ();
# type AdminUser = rocket::http::Method;
# type User = rocket::http::Method;
# #[get("/login")]
# fn login() -> Template { /* .. */ }

use rocket::response::Redirect;

#[get("/admin", rank = 2)]
fn admin_panel_user(user: Option<User>) -> Result<&'static str, Redirect> {
    let user = user.ok_or_else(|| Redirect::to(uri!(login)))?;
    Ok("Sorry, you must be an administrator to access this page.")
}
```

If the `User` guard forwards or fails, the `Option` will be `None`. If it
succeeds, it will be `Some(User)`.

For guards that may fail (and not just forward), the `Result<T, E>` guard allows
retrieving the error type `E` on error. As an example, when the
[`mtls::Certificate`] type fails, it reports the reason in an [`mtls::Error`]
type. The value can be retrieved in a handler by using a `Result<Certificate,
Error>` guard:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::mtls;

#[get("/login")]
fn login(cert: Result<mtls::Certificate, mtls::Error>) {
    match cert {
        Ok(cert) => { /* guard succeeded! value in `cert` */ },
        Err(e) => { /* guard failed. error in `e` */ },
    }
}
```

It's important to note that `Result<T, E>` forwards if `T` forwards. You can,
however, chain both catching responders to determine if a guard `T` forwards or
fails, and retrieve the error if it fails, with `Option<Result<T, E>>`:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

use rocket::mtls;

#[get("/login")]
fn login(cert: Option<Result<mtls::Certificate, mtls::Error>>) {
    match cert {
        Some(Ok(cert)) => { /* guard succeeded! value in `cert` */ },
        Some(Err(e)) => { /* guard failed. error in `e` */ },
        None => { /* guard forwarded */ },
    }
}
```

[`mtls::Certificate`]: @api/master/rocket/mtls/struct.Certificate.html
[`mtls::Error`]: @api/master/rocket/mtls/enum.Error.html

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

[cookies example]: @git/master/examples/cookies

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
rocket = { version = "0.6.0-dev", features = ["secrets"] }
```

The API for retrieving, adding, and removing private cookies is identical except
that most methods are suffixed with `_private`. These methods are:
[`get_private`], [`add_private`], and [`remove_private`]. An example of their
usage is below:

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
    cookies.remove_private("user_id");
    Flash::success(Redirect::to("/"), "Successfully logged out.")
}
```

[`CookieJar::add()`]: @api/master/rocket/http/struct.CookieJar.html#method.add

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

For more information on configuration, see the [Configuration] section of the
guide.

[`get_private`]: @api/master/rocket/http/struct.CookieJar.html#method.get_private
[`add_private`]: @api/master/rocket/http/struct.CookieJar.html#method.add_private
[`remove_private`]: @api/master/rocket/http/struct.CookieJar.html#method.remove_private
[Configuration]: ../configuration/

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

# type User = String;

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

[`ContentType::parse_flexible()`]: @api/master/rocket/http/struct.ContentType.html#method.parse_flexible

## Body Data

Body data processing, like much of Rocket, is type directed. To indicate that a
handler expects body data, annotate it with `data = "<param>"`, where `param` is
an argument in the handler. The argument's type must implement the [`FromData`]
trait. It looks like this, where `T` is assumed to implement `FromData`:

```rust
# #[macro_use] extern crate rocket;

# type T = String;

#[post("/", data = "<input>")]
fn new(input: T) { /* .. */ }
```

Any type that implements [`FromData`] is also known as _a data guard_.

[`FromData`]: @api/master/rocket/data/trait.FromData.html

### JSON

The [`Json<T>`](@api/master/rocket/serde/json/struct.Json.html) guard deserializes body
data as JSON. The only condition is that the generic type `T` implements the
`Deserialize` trait from [`serde`](https://serde.rs).

```rust
# #[macro_use] extern crate rocket;

use rocket::serde::{Deserialize, json::Json};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Task<'r> {
    description: &'r str,
    complete: bool
}

#[post("/todo", data = "<task>")]
fn new(task: Json<Task<'_>>) { /* .. */ }
```

! warning: Using Rocket's `serde` derive re-exports requires a bit more effort.

  For convenience, Rocket re-exports `serde`'s `Serialize` and `Deserialize`
  traits and derive macros from `rocket::serde`. However, due to Rust's limited
  support for derive macro re-exports, using the re-exported derive macros
  requires annotating structures with `#[serde(crate = "rocket::serde")]`. If
  you'd like to avoid this extra annotation, you must depend on `serde` directly
  via your crate's `Cargo.toml`:

  ```toml
  serde = { version = "1.0", features = ["derive"] }
  ```

  We always use the extra annotation in the guide, but you may prefer the
  alternative.

See the [JSON example](@git/master/examples/serialization/src/json.rs) on GitHub for a
complete example.

! note: JSON support requires enabling Rocket's `json` feature flag.

  Rocket intentionally places JSON support, as well support for other data
  formats and features, behind feature flags. See [the api
  docs](@api/master/rocket/#features) for a list of available features. The `json`
  feature can be enabled in the `Cargo.toml`:

  ```toml
  rocket = { version = "0.6.0-dev", features = ["json"] }
  ```

### Temporary Files

The [`TempFile`] data guard streams data directly to a temporary file which can
then be persisted. It makes accepting file uploads trivial:

```rust
# #[macro_use] extern crate rocket;

use rocket::fs::TempFile;

#[post("/upload", format = "plain", data = "<file>")]
async fn upload(mut file: TempFile<'_>) -> std::io::Result<()> {
    # let permanent_location = "/tmp/perm.txt";
    file.persist_to(permanent_location).await
}
```

[`TempFile`]: @api/master/rocket/fs/enum.TempFile.html

### Streaming

Sometimes you just want to handle incoming data directly. For example, you might
want to stream the incoming data to some sink. Rocket makes this as simple as
possible via the [`Data`](@api/master/rocket/data/struct.Data.html) type:

```rust
# #[macro_use] extern crate rocket;

use rocket::tokio;

use rocket::data::{Data, ToByteUnit};

#[post("/debug", data = "<data>")]
async fn debug(data: Data<'_>) -> std::io::Result<()> {
    // Stream at most 512KiB of the body data to stdout.
    data.open(512.kibibytes())
        .stream_to(tokio::io::stdout())
        .await?;

    Ok(())
}
```

The route above accepts any `POST` request to the `/debug` path. At most 512KiB
of the incoming is streamed out to `stdout`. If the upload fails, an error
response is returned. The handler above is complete. It really is that simple!

! note: Rocket requires setting limits when reading incoming data.

  To aid in preventing DoS attacks, Rocket requires you to specify, as a
  [`ByteUnit`](@api/master/rocket/data/struct.ByteUnit.html), the amount of data you're
  willing to accept from the client when `open`ing a data stream. The
  [`ToByteUnit`](@api/master/rocket/data/trait.ToByteUnit.html) trait makes specifying
  such a value as idiomatic as `128.kibibytes()`.

## Forms

Forms are one of the most common types of data handled in web applications, and
Rocket makes handling them easy. Rocket supports both `multipart` and
`x-www-form-urlencoded` forms out of the box, enabled by the [`Form`] data guard
and derivable [`FromForm`] trait.

Say your application is processing a form submission for a new todo `Task`. The
form contains two fields: `complete`, a checkbox, and `type`, a text field. You
can easily handle the form request in Rocket as follows:

```rust
# #[macro_use] extern crate rocket;

use rocket::form::Form;

#[derive(FromForm)]
struct Task<'r> {
    complete: bool,
    r#type: &'r str,
}

#[post("/todo", data = "<task>")]
fn new(task: Form<Task<'_>>) { /* .. */ }
```

[`Form`] is data guard as long as its generic parameter implements the
[`FromForm`] trait. In the example, we've derived the `FromForm` trait
automatically for `Task`. `FromForm` can be derived for any structure whose
fields implement [`FromForm`], or equivalently, [`FromFormField`].

If a `POST /todo` request arrives, the form data will automatically be parsed
into the `Task` structure. If the data that arrives isn't of the correct
Content-Type, the request is forwarded. If the data doesn't parse or is simply
invalid, a customizable error is returned. As before, a forward or error can
be caught by using the `Option` and `Result` types:

```rust
# use rocket::{post, form::Form};
# type Task<'r> = &'r str;

#[post("/todo", data = "<task>")]
fn new(task: Option<Form<Task<'_>>>) { /* .. */ }
```

### Multipart

Multipart forms are handled transparently, with no additional effort. Most
`FromForm` types can parse themselves from the incoming data stream. For
example, here's a form and route that accepts a multipart file upload using
[`TempFile`]:

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
fn upload_form(upload: Form<Upload<'_>>) { /* .. */ }
```

[`Form`]: @api/master/rocket/form/struct.Form.html
[`FromForm`]: @api/master/rocket/form/trait.FromForm.html
[`FromFormField`]: @api/master/rocket/form/trait.FromFormField.html

### Parsing Strategy

Rocket's `FromForm` parsing is _lenient_ by default: a `Form<T>` will parse
successfully from an incoming form even if it contains extra, duplicate, or
missing fields. Extras or duplicates are ignored -- no validation or parsing of
the fields occurs -- and missing fields are filled with defaults when available.
To change this behavior and make form parsing _strict_, use the
[`Form<Strict<T>>`] data type, which emits errors if there are any extra or
missing fields, irrespective of defaults.

You can use a `Form<Strict<T>>` anywhere you'd use a `Form<T>`. Its generic
parameter is also required to implement `FromForm`. For instance, we can simply
replace `Form<T>` with `Form<Strict<T>>` above to get strict parsing:

```rust
# #[macro_use] extern crate rocket;

use rocket::form::{Form, Strict};

# #[derive(FromForm)] struct Task<'r> { complete: bool, description: &'r str, }

#[post("/todo", data = "<task>")]
fn new(task: Form<Strict<Task<'_>>>) { /* .. */ }
```

`Strict` can also be used to make individual fields strict while keeping the
overall structure and remaining fields lenient:

```rust
# #[macro_use] extern crate rocket;
# use rocket::form::{Form, Strict};

#[derive(FromForm)]
struct Input {
    required: Strict<bool>,
    uses_default: bool
}

#[post("/", data = "<input>")]
fn new(input: Form<Input>) { /* .. */ }
```

[`Lenient`] is the _lenient_ analog to `Strict`, which forces parsing to be
lenient. `Form` is lenient by default, so a `Form<Lenient<T>>` is redundant, but
`Lenient` can be used to overwrite a strict parse as lenient:
`Option<Lenient<T>>`.

[`Form<Strict<T>>`]: @api/master/rocket/form/struct.Strict.html
[`Lenient`]: @api/master/rocket/form/struct.Lenient.html

### Defaults

A form guard may specify a default value to use when a field is missing. The
default value is used only when parsing is _lenient_. When _strict_, all errors,
including missing fields, are propagated directly.

Some types with defaults include `bool`, which defaults to `false`, useful for
checkboxes, `Option<T>`, which defaults to `None`, and [`form::Result`], which
defaults to `Err(Missing)` or otherwise collects errors in an `Err` of
[`Errors<'_>`]. Defaulting guards can be used just like any other form guard:

```rust
# use rocket::form::FromForm;
use rocket::form::{self, Errors};

#[derive(FromForm)]
struct MyForm<'v> {
    maybe_string: Option<&'v str>,
    ok_or_error: form::Result<'v, Vec<&'v str>>,
    here_or_false: bool,
}
# rocket_docs_tests::assert_form_parses_ok!(MyForm, "");
```

The default can be overridden or unset using the `#[field(default = expr)]`
field attribute. If `expr` is not literally `None`, the parameter sets the
default value of the field to be `expr.into()`. If `expr` _is_ `None`, the
parameter _unsets_ the default value of the field, if any.

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    // Set the default value to be `"hello"`.
    //
    // Note how an `&str` is automatically converted into a `String`.
    #[field(default = "hello")]
    greeting: String,
    // Remove the default value of `false`, requiring all parses of `MyForm`
    // to contain an `is_friendly` field.
    #[field(default = None)]
    is_friendly: bool,
}
```

See the [`FromForm` derive] documentation for full details on the `default`
attribute parameter as well documentation on the more expressive `default_with`
parameter option.

[`Errors<'_>`]: @api/master/rocket/form/struct.Errors.html
[`form::Result`]: @api/master/rocket/form/type.Result.html
[`FromForm` derive]: @api/master/rocket/derive.FromForm.html

### Field Renaming

By default, Rocket matches the name of an incoming form field to the name of a
structure field. While this behavior is typical, it may also be desired to use
different names for form fields and struct fields while still parsing as
expected. You can ask Rocket to look for a different form field for a given
structure field by using one or more `#[field(name = "name")]` or `#[field(name
= uncased("name")]` field annotation. The `uncased` variant case-insensitively
matches field names.

As an example, say that you're writing an application that receives data from an
external service. The external service `POST`s a form with a field named
`first-Name` which you'd like to write as `first_name` in Rust. Such a form
structure can be written as:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct External<'r> {
    #[field(name = "first-Name")]
    first_name: &'r str
}
```

If you want to accept both `firstName` case-insensitively as well as
`first_name` case-sensitively, you'll need to use two annotations:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct External<'r> {
    #[field(name = uncased("firstName"))]
    #[field(name = "first_name")]
    first_name: &'r str
}
```

This will match any casing of `firstName` including `FirstName`, `firstname`,
`FIRSTname`, and so on, but only match exactly on `first_name`.

If instead you wanted to match any of `first-name`, `first_name` or `firstName`,
in each instance case-insensitively, you would write:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct External<'r> {
    #[field(name = uncased("first-name"))]
    #[field(name = uncased("first_name"))]
    #[field(name = uncased("firstname"))]
    first_name: &'r str
}
```

Cased and uncased renamings can be mixed and matched, and any number of
renamings is allowed. Rocket will emit an error at compile-time if field names
conflict, preventing ambiguous parsing at runtime.

### Ad-Hoc Validation

Fields of forms can be easily ad-hoc validated via the `#[field(validate)]`
attribute. As an example, consider a form field `age: u16` which we'd like to
ensure is greater than `21`. The following structure accomplishes this:

```rust
# #[macro_use] extern crate rocket;

#[derive(FromForm)]
struct Person {
    #[field(validate = range(21..))]
    age: u16
}
```

The expression `range(21..)` is a call to [`form::validate::range`]. Rocket
passes a borrow of the attributed field, here `self.age`, as the first parameter
to the function call. The rest of the fields are pass as written in the
expression.

Any function in the [`form::validate`] module can be called, and other fields of
the form can be passed in by using `self.$field` where `$field` is the name of
the field in the structure. You can also apply more than one validation to a
field by using multiple attributes. For example, the following form validates
that the value of the field `confirm` is equal to the value of the field `value`
and that it doesn't contain `no`:

```rust
# #[macro_use] extern crate rocket;

#[derive(FromForm)]
struct Password<'r> {
    #[field(name = "password")]
    value: &'r str,
    #[field(validate = eq(self.value))]
    #[field(validate = omits("no"))]
    confirm: &'r str,
}
```

[`form::validate`]: @api/master/rocket/form/validate/index.html
[`form::validate::range`]: @api/master/rocket/form/validate/fn.range.html
[`form::Result`]: @api/master/rocket/form/type.Result.html
[`Errors<'_>`]: @api/master/rocket/form/error/struct.Errors.html

In reality, the expression after `validate =` can be _any_ expression as long as
it evaluates to a value of type `Result<(), Errors<'_>>` (aliased by
[`form::Result`]), where an `Ok` value means that validation was successful while
an `Err` of [`Errors<'_>`] indicates the error(s) that occurred. For instance, if
you wanted to implement an ad-hoc Luhn validator for credit-card-like numbers,
you might write:

```rust
# #[macro_use] extern crate rocket;

use rocket::time::Date;
use rocket::form::{self, Error};

#[derive(FromForm)]
struct CreditCard {
    #[field(validate = luhn(self.cvv, &self.expiration))]
    number: u64,
    #[field(validate = range(..9999))]
    cvv: u16,
    expiration: Date,
}

fn luhn<'v>(number: &u64, cvv: u16, exp: &Date) -> form::Result<'v, ()> {
    # let valid = false;
    if !valid {
        Err(Error::validation("invalid credit card number"))?;
    }

    Ok(())
}
```

If a field's validation doesn't depend on other fields (validation is _local_),
it is validated prior to those fields that do. For `CreditCard`, `cvv` and
`expiration` will be validated prior to `number`.

### Wrapping Validators

If a particular validation is applied in more than one place, prefer creating a
type that encapsulates and represents the validated value. For example, if your
application often validates `age` fields, consider creating a custom `Age` form
guard that always applies the validation:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
#[field(validate = range(18..150))]
struct Age(u16);
```

This approach is also useful when a custom validator already exists in some
other form. For instance, the following example leverages [`try_with`] and an
existing `FromStr` implementation on a `Token` type to validate a string:

```rust
# use rocket::form::FromForm;

# impl FromStr for Token<'_> {
#     type Err = &'static str;
#     fn from_str(s: &str) -> Result<Self, Self::Err> { todo!() }
# }

use std::str::FromStr;

#[derive(FromForm)]
#[field(validate = try_with(|s| Token::from_str(s)))]
struct Token<'r>(&'r str);
```

[`try_with`]: @api/master/rocket/form/validate/fn.try_with.html

### Collections

Rocket's form support allows your application to express _any_ structure with
_any_ level of nesting and collection, eclipsing the expressivity offered by any
other web framework. To parse into these structures, Rocket separates a field's
name into "keys" by the delimiters `.` and `[]`, each of which in turn is
separated into "indices" by `:`. In other words, a name has keys and a key has
indices, each a strict subset of its parent. This is depicted in the example
below with two form fields:

```html
food.bart[bar:foo].blam[0_0][1000]=some-value&another_field=another_val
|-------------------------------|   name
|--| |--| |-----|  |--| |-|  |--|   keys
|--| |--| |-| |-|  |--| |-|  |--|   indices
```

Rocket _pushes_ form fields to `FromForm` types as they arrive. The type then
operates on _one_ key (and all of its indices) at a time and _shifts_ to the
next `key`, from left-to-right, before invoking any other `FromForm` types with
the rest of the field. A _shift_ encodes a nested structure while indices allows
for structures that need more than one value to allow indexing.

! note: A `.` after a `[]` is optional.

  The form field name `a[b]c` is exactly equivalent to `a[b].c`. Likewise, the
  form field name `.a` is equivalent to `a`.

### Nesting

Form structs can be nested:

```rust
use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm<'r> {
    owner: Person<'r>,
    pet: Pet<'r>,
}

#[derive(FromForm)]
struct Person<'r> {
    name: &'r str
}

#[derive(FromForm)]
struct Pet<'r> {
    name: &'r str,
    #[field(validate = eq(true))]
    good_pet: bool,
}
```

To parse into a `MyForm`, a form with the following fields must be submitted:

  * `owner.name` - string
  * `pet.name` - string
  * `pet.good_pet` - boolean

Such a form, URL-encoded, may look like:

```rust
# use rocket::form::FromForm;
# use rocket_docs_tests::{assert_form_parses, assert_not_form_parses};
# #[derive(FromForm, Debug, PartialEq)] struct MyForm { owner: Person, pet: Pet, }
# #[derive(FromForm, Debug, PartialEq)] struct Person { name: String }
# #[derive(FromForm, Debug, PartialEq)] struct Pet { name: String, good_pet: bool, }

# assert_form_parses! { MyForm,
"owner.name=Bob&pet.name=Sally&pet.good_pet=on",
# "owner.name=Bob&pet.name=Sally&pet.good_pet=yes",
# "owner.name=Bob&pet.name=Sally&pet.good_pet=on",
# "pet.name=Sally&owner.name=Bob&pet.good_pet=on",
# "pet.name=Sally&pet.good_pet=on&owner.name=Bob",
# =>

// ...which parses as this struct.
MyForm {
    owner: Person {
        name: "Bob".into()
    },
    pet: Pet {
        name: "Sally".into(),
        good_pet: true,
    }
}
# };
```

Note that `.` is used to separate each field. Identically, `[]` can be used in
place of or in addition to `.`:

```rust
# use rocket::form::FromForm;
# use rocket_docs_tests::{assert_form_parses, assert_not_form_parses};
# #[derive(FromForm, Debug, PartialEq)] struct MyForm { owner: Person, pet: Pet, }
# #[derive(FromForm, Debug, PartialEq)] struct Person { name: String }
# #[derive(FromForm, Debug, PartialEq)] struct Pet { name: String, good_pet: bool, }

// All of these are identical to the previous...
# assert_form_parses! { MyForm,
"owner[name]=Bob&pet[name]=Sally&pet[good_pet]=on",
"owner[name]=Bob&pet[name]=Sally&pet.good_pet=on",
"owner.name=Bob&pet[name]=Sally&pet.good_pet=on",
"pet[name]=Sally&owner.name=Bob&pet.good_pet=on",
# =>

// ...and thus parse as this struct.
MyForm {
    owner: Person {
        name: "Bob".into()
    },
    pet: Pet {
        name: "Sally".into(),
        good_pet: true,
    }
}
# };
```

Any level of nesting is allowed.

### Vectors

A form can also contain sequences:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    numbers: Vec<usize>,
}
```

To parse into a `MyForm`, a form with the following fields must be submitted:

  * `numbers[$k]` - usize (or equivalently, `numbers.$k`)

...where `$k` is the "key" used to determine whether to push the rest of the
field to the last element in the vector or create a new one. If the key is the
same as the previous key seen by the vector, then the field's value is pushed to
the last element. Otherwise, a new element is created. The actual value of `$k`
is irrelevant: it is only used for comparison, has no semantic meaning, and is
not remembered by `Vec`. The special blank key is never equal to any other key.

Consider the following examples.

```rust
# use rocket::form::FromForm;
# use rocket_docs_tests::{assert_form_parses, assert_not_form_parses};
# #[derive(FromForm, PartialEq, Debug)] struct MyForm { numbers: Vec<usize>, }
// These form strings...
# assert_form_parses! { MyForm,
"numbers[]=1&numbers[]=2&numbers[]=3",
"numbers[a]=1&numbers[b]=2&numbers[c]=3",
"numbers[a]=1&numbers[b]=2&numbers[a]=3",
"numbers[]=1&numbers[b]=2&numbers[c]=3",
"numbers.0=1&numbers.1=2&numbers[c]=3",
"numbers=1&numbers=2&numbers=3",
# =>

// ...parse as this struct:
MyForm {
    numbers: vec![1 ,2, 3]
}
# };

// These, on the other hand...
# assert_form_parses! { MyForm,
"numbers[0]=1&numbers[0]=2&numbers[]=3",
"numbers[]=1&numbers[b]=3&numbers[b]=2",
# =>

// ...parse as this struct:
MyForm {
    numbers: vec![1, 3]
}
# };
```

You might be surprised to see the last example,
`"numbers=1&numbers=2&numbers=3"`, in the first list. This is equivalent to the
previous examples as the "key" seen by the `Vec` (everything after `numbers`) is
empty. Thus, `Vec` pushes to a new `usize` for every field. `usize`, like all
types that implement `FromFormField`, discard duplicate and extra fields when
parsed leniently, keeping only the _first_ field.

### Nesting in Vectors

Any `FromForm` type can appear in a sequence:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    name: String,
    pets: Vec<Pet>,
}

#[derive(FromForm)]
struct Pet {
    name: String,
    #[field(validate = eq(true))]
    good_pet: bool,
}
```

To parse into a `MyForm`, a form with the following fields must be submitted:

  * `name` - string
  * `pets[$k].name` - string
  * `pets[$k].good_pet` - boolean

Examples include:

```rust
# use rocket::form::FromForm;
# use rocket_docs_tests::{assert_form_parses, assert_not_form_parses};
# #[derive(FromForm, Debug, PartialEq)] struct MyForm { name: String, pets: Vec<Pet>, }
# #[derive(FromForm, Debug, PartialEq)] struct Pet { name: String, good_pet: bool, }
// These form strings...
assert_form_parses! { MyForm,
"name=Bob&pets[0].name=Sally&pets[0].good_pet=on",
"name=Bob&pets[sally].name=Sally&pets[sally].good_pet=yes",
# =>

// ...parse as this struct:
MyForm {
    name: "Bob".into(),
    pets: vec![Pet { name: "Sally".into(), good_pet: true }],
}
# };

// These, on the other hand, fail to parse:
# assert_not_form_parses! { MyForm,
"name=Bob&pets[0].name=Sally&pets[1].good_pet=on",
"name=Bob&pets[].name=Sally&pets[].good_pet=on",
# };
```

### Nested Vectors

Since vectors are `FromForm` themselves, they can appear inside of vectors:

```rust
# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    v: Vec<Vec<usize>>,
}
```

The rules are exactly the same.

```rust
# use rocket::form::FromForm;
# use rocket_docs_tests::assert_form_parses;
# #[derive(FromForm, Debug, PartialEq)] struct MyForm { v: Vec<Vec<usize>>, }
# assert_form_parses! { MyForm,
"v=1&v=2&v=3" => MyForm { v: vec![vec![1], vec![2], vec![3]] },
"v[][]=1&v[][]=2&v[][]=3" => MyForm { v: vec![vec![1], vec![2], vec![3]] },
"v[0][]=1&v[0][]=2&v[][]=3" => MyForm { v: vec![vec![1, 2], vec![3]] },
"v[][]=1&v[0][]=2&v[0][]=3" => MyForm { v: vec![vec![1], vec![2, 3]] },
"v[0][]=1&v[0][]=2&v[0][]=3" => MyForm { v: vec![vec![1, 2, 3]] },
"v[0][0]=1&v[0][0]=2&v[0][]=3" => MyForm { v: vec![vec![1, 3]] },
"v[0][0]=1&v[0][0]=2&v[0][0]=3" => MyForm { v: vec![vec![1]] },
# };
```

### Maps

A form can also contain maps:

```rust
# use rocket::form::FromForm;
use std::collections::HashMap;

#[derive(FromForm)]
struct MyForm {
    ids: HashMap<String, usize>,
}
```

To parse into a `MyForm`, a form with the following fields must be submitted:

  * `ids[$string]` - usize (or equivalently, `ids.$string`)

...where `$string` is the "key" used to determine which value in the map to push
the rest of the field to. Unlike with vectors, the key _does_ have a semantic
meaning and _is_ remembered, so ordering of fields is inconsequential: a given
string `$string` always maps to the same element.

As an example, the following are equivalent and all parse to `{ "a" => 1, "b" =>
2 }`:

```rust
# use std::collections::HashMap;
#
# use rocket::form::FromForm;
# use rocket_docs_tests::{map, assert_form_parses};
#
# #[derive(Debug, PartialEq, FromForm)]
# struct MyForm {
#     ids: HashMap<String, usize>,
# }
// These form strings...
# assert_form_parses! { MyForm,
"ids[a]=1&ids[b]=2",
"ids[b]=2&ids[a]=1",
"ids[a]=1&ids[a]=2&ids[b]=2",
"ids.a=1&ids.b=2",
# =>

// ...parse as this struct:
MyForm {
    ids: map! {
        "a" => 1usize,
        "b" => 2usize,
    }
}
# };
```

Both the key and value of a `HashMap` can be any type that implements
`FromForm`. Consider a value representing another structure:

```rust
# use std::collections::HashMap;

# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    ids: HashMap<usize, Person>,
}

#[derive(FromForm)]
struct Person {
    name: String,
    age: usize
}
```

To parse into a `MyForm`, a form with the following fields must be submitted:

  * `ids[$usize].name` - string
  * `ids[$usize].age` - usize

Examples include:

```rust
# use std::collections::HashMap;
#
# use rocket::form::FromForm;
# use rocket_docs_tests::{map, assert_form_parses};
#

# #[derive(FromForm, Debug, PartialEq)] struct MyForm { ids: HashMap<usize, Person>, }
# #[derive(FromForm, Debug, PartialEq)] struct Person { name: String, age: usize }

// These form strings...
# assert_form_parses! { MyForm,
"ids[0]name=Bob&ids[0]age=3&ids[1]name=Sally&ids[1]age=10",
"ids[0]name=Bob&ids[1]age=10&ids[1]name=Sally&ids[0]age=3",
"ids[0]name=Bob&ids[1]name=Sally&ids[0]age=3&ids[1]age=10",
# =>

// ...which parse as this struct:
MyForm {
    ids: map! {
        0usize => Person { name: "Bob".into(), age: 3 },
        1usize => Person { name: "Sally".into(), age: 10 },
    }
}
# };
```

Now consider the following structure where both the key and value represent
structures:

```rust
# use std::collections::HashMap;

# use rocket::form::FromForm;

#[derive(FromForm)]
struct MyForm {
    m: HashMap<Person, Pet>,
}

#[derive(FromForm, PartialEq, Eq, Hash)]
struct Person {
    name: String,
    age: usize
}

#[derive(FromForm)]
struct Pet {
    wags: bool
}
```

! warning: The `HashMap` key type, here `Person`, must implement `Eq + Hash`.

Since the key is a collection, here `Person`, it must be built up from multiple
fields. This requires being able to specify via the form field name that the
field's value corresponds to a key in the map. The is done with the syntax
`k:$key` which indicates that the field corresponds to the `k`ey named `$key`.
Thus, to parse into a `MyForm`, a form with the following fields must be
submitted:

  * `m[k:$key].name` - string
  * `m[k:$key].age` - usize
  * `m[$key].wags` or `m[v:$key].wags`  - boolean

! note: The syntax `v:$key` also exists.

  The shorthand `m[$key]` is equivalent to `m[v:$key]`.

Note that `$key` can be _anything_: it is simply a symbolic identifier for a
key/value pair in the map and has no bearing on the actual values that will be
parsed into the map.

Examples include:

```rust
# use std::collections::HashMap;
#
# use rocket::form::FromForm;
# use rocket_docs_tests::{map, assert_form_parses};
#

# #[derive(FromForm, Debug, PartialEq)] struct MyForm { m: HashMap<Person, Pet>, }
# #[derive(FromForm, Debug, PartialEq, Eq, Hash)] struct Person { name: String, age: usize }
# #[derive(FromForm, Debug, PartialEq)] struct Pet { wags: bool }

// These form strings...
# assert_form_parses! { MyForm,
"m[k:alice]name=Alice&m[k:alice]age=30&m[v:alice].wags=no",
"m[k:alice]name=Alice&m[k:alice]age=30&m[alice].wags=no",
"m[k:123]name=Alice&m[k:123]age=30&m[123].wags=no",
# =>

// ...which parse as this struct:
MyForm {
    m: map! {
        Person { name: "Alice".into(), age: 30 } => Pet { wags: false }
    }
}
# };

// While this longer form string...
# assert_form_parses! { MyForm,
"m[k:a]name=Alice&m[k:a]age=40&m[a].wags=no&\
m[k:b]name=Bob&m[k:b]age=72&m[b]wags=yes&\
m[k:cat]name=Katie&m[k:cat]age=12&m[cat]wags=yes",
# =>

// ...parses as this struct:
MyForm {
    m: map! {
        Person { name: "Alice".into(), age: 40 } => Pet { wags: false },
        Person { name: "Bob".into(), age: 72 } => Pet { wags: true },
        Person { name: "Katie".into(), age: 12 } => Pet { wags: true },
    }
}
# };
```

### Arbitrary Collections

_Any_ collection can be expressed with any level of arbitrary nesting, maps, and
sequences. Consider the extravagantly contrived type:

```rust
use std::collections::{BTreeMap, HashMap};
# use rocket::form::FromForm;

#[derive(FromForm, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Person {
    name: String,
    age: usize
}

# type Foo =
HashMap<Vec<BTreeMap<Person, usize>>, HashMap<usize, Person>>
# ;
# /*
|-[k:$k1]-----------|------|------| |-[$k1]-----------------|
     |---[$i]-------|------|------|         |-[k:$j]*|
           |-[k:$k2]|------|                 ~~[$j]~~|name*|
                    |-name*|                 ~~[$j]~~|age-*|
                    |-age*-|
           |~~~~~~~~~~~~~~~|v:$k2*|
# */
```

! warning: The `BTreeMap` key type, here `Person`, must implement `Ord`.

As illustrated above with `*` marking terminals, we need the following form
fields for this structure:

  * `[k:$k1][$i][k:$k2]name` - string
  * `[k:$k1][$i][k:$k2]age` - usize
  * `[k:$k1][$i][$k2]` - usize
  * `[$k1][k:$j]` - usize
  * `[$k1][$j]name` - string
  * `[$k1][$j]age` - string

Where we have the following symbolic keys:

  * `$k1`: symbolic name of the top-level key
  * `$i`: symbolic name of the vector index
  * `$k2`: symbolic name of the sub-level  (`BTreeMap`) key
  * `$j`: symbolic name and/or value top-level value's key

```rust
# use std::collections::BTreeMap;
# use std::collections::HashMap;
#
# use rocket::form::FromForm;
# use rocket_docs_tests::{map, bmap, assert_form_parses};
# #[derive(FromForm, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
# struct Person { name: String, age: usize }

type Foo = HashMap<Vec<BTreeMap<Person, usize>>, HashMap<usize, Person>>;

// This (long, contrived) form string...
# assert_form_parses! { Foo,
"[k:top_key][i][k:sub_key]name=Bobert&\
[k:top_key][i][k:sub_key]age=22&\
[k:top_key][i][sub_key]=1337&\
[top_key][7]name=Builder&\
[top_key][7]age=99",

// We could also set the top-level value's key explicitly:
// [top_key][k:7]=7
# "[k:top_key][i][k:sub_key]name=Bobert&\
# [k:top_key][i][k:sub_key]age=22&\
# [top_key][k:7]=7&\
# [k:top_key][i][sub_key]=1337&\
# [top_key][7]name=Builder&\
# [top_key][7]age=99",
# =>

// ...parses as this (long, contrived) map:
map! {
    vec![bmap! {
        Person { name: "Bobert".into(), age: 22 } => 1337usize,
    }]
    =>
    map! {
        7usize => Person { name: "Builder".into(), age: 99 }
    }
}
# };
```

### Context

The [`Contextual`] form guard acts as a proxy for any other form guard,
recording all submitted form values and produced errors and associating them
with their corresponding field name. `Contextual` is particularly useful for
rendering forms with previously submitted values and errors associated with form
input.

To retrieve the context for a form, use `Form<Contextual<'_, T>>` as a data
guard, where `T` implements `FromForm`. The `context` field contains the form's
[`Context`]:

```rust
# use rocket::post;
# type T = String;

use rocket::form::{Form, Contextual};

#[post("/submit", data = "<form>")]
fn submit(form: Form<Contextual<'_, T>>) {
    if let Some(ref value) = form.value {
        // The form parsed successfully. `value` is the `T`.
    }

    // We can retrieve raw field values and errors.
    let raw_id_value = form.context.field_value("id");
    let id_errors = form.context.field_errors("id");
}
```

`Context` is nesting-aware for errors. When `Context` is queried for errors for
a field named `foo.bar`, it returns errors for fields that are a prefix of
`foo.bar`, namely `foo` and `foo.bar`. Similarly, if queried for errors for a
field named `foo.bar.baz`, errors for field `foo`, `foo.bar`, and `foo.bar.baz`
will be returned.

`Context` serializes as a map, so it can be rendered in templates that require
`Serialize` types. See [`Context`] for details about its serialization format.
The [forms example], too, makes use of form contexts, as well as every other
forms feature.

[`Contextual`]: @api/master/rocket/form/struct.Contextual.html
[`Context`]: @api/master/rocket/form/struct.Context.html
[forms example]: @git/master/examples/forms

## Query Strings

Query strings are URL-encoded forms that appear in the URL of a request. Query
parameters are declared like path parameters but otherwise handled like regular
URL-encoded form fields. The table below summarizes the analogy:

| Path Syntax | Query Syntax | Path Type Bound  | Query Type Bound |
|-------------|--------------|------------------|------------------|
| `<param>`   | `<param>`    | [`FromParam`]    | [`FromForm`]     |
| `<param..>` | `<param..>`  | [`FromSegments`] | [`FromForm`]     |
| `static`    | `static`     | N/A              | N/A              |

Because dynamic parameters are form types, they can be single values,
collections, nested collections, or anything in between, just like any other
form field.

### Static Parameters

A request matches a route _iff_ its query string contains all of the static
parameters in the route's query string. A route with a static parameter `param`
(any UTF-8 text string) in a query will only match requests with that exact path
segment in its query string.

! note: This is truly an _iff_!

  Only the static parameters in query route string affect routing. Dynamic
  parameters are allowed to be missing by default.


For example, the route below will match requests with path `/` and _at least_
the query segments `hello` and `cat=♥`:

```rust
# #[macro_use] extern crate rocket;

#[get("/?hello&cat=♥")]
fn cats() -> &'static str {
    "Hello, kittens!"
}

// The following GET requests match `cats`. `%E2%99%A5` is encoded `♥`.
# let status = rocket_docs_tests::client(routes![cats]).get(
"/?cat=%E2%99%A5&hello"
# ).dispatch().status();
# assert_eq!(status, rocket::http::Status::Ok);
# let status = rocket_docs_tests::client(routes![cats]).get(
"/?hello&cat=%E2%99%A5"
# ).dispatch().status();
# assert_eq!(status, rocket::http::Status::Ok);
# let status = rocket_docs_tests::client(routes![cats]).get(
"/?dogs=amazing&hello&there&cat=%E2%99%A5"
# ).dispatch().status();
# assert_eq!(status, rocket::http::Status::Ok);
```

### Dynamic Parameters

A single dynamic parameter of `<param>` acts identically to a form field
declared as `param`. In particular, Rocket will expect the query form to contain
a field with key `param` and push the shifted field to the `param` type. As with
forms, default values are used when parsing fails. The example below illustrates
this with a single value `name`, a collection `color`, a nested form `person`,
and an `other` value that will default to `None`:

```rust
# #[macro_use] extern crate rocket;

#[derive(Debug, PartialEq, FromFormField)]
enum Color {
    Red,
    Blue,
    Green
}

#[derive(Debug, PartialEq, FromForm)]
struct Pet<'r> {
  name: &'r str,
  age: usize,
}

#[derive(Debug, PartialEq, FromForm)]
struct Person<'r> {
  pet: Pet<'r>,
}

#[get("/?<name>&<color>&<person>&<other>")]
fn hello(name: &str, color: Vec<Color>, person: Person<'_>, other: Option<usize>) {
    assert_eq!(name, "George");
    assert_eq!(color, [Color::Red, Color::Green, Color::Green, Color::Blue]);
    assert_eq!(other, None);
    assert_eq!(person, Person {
      pet: Pet { name: "Fi Fo Alex", age: 1 }
    });
}

// A request with these query segments matches as above.
# let status = rocket_docs_tests::client(routes![hello]).get("/?\
name=George&\
color=red&\
color=green&\
person.pet.name=Fi+Fo+Alex&\
color=green&\
person.pet.age=1&\
color=blue&\
extra=yes\
# ").dispatch().status();
# assert_eq!(status, rocket::http::Status::Ok);
```

Note that, like forms, parsing is field-ordering insensitive and lenient by
default.

### Trailing Parameter

A trailing dynamic parameter of `<param..>` collects all of the query segments
that don't otherwise match a declared static or dynamic parameter. In other
words, the otherwise unmatched segments are pushed, unshifted, to the
`<param..>` type:

```rust
# #[macro_use] extern crate rocket;

use rocket::form::Form;

#[derive(FromForm)]
struct User<'r> {
    name: &'r str,
    active: bool,
}

#[get("/?hello&<id>&<user..>")]
fn user(id: usize, user: User<'_>) {
    assert_eq!(id, 1337);
    assert_eq!(user.name, "Bob Smith");
    assert_eq!(user.active, true);
}

// A request with these query segments matches as above.
# let status = rocket_docs_tests::client(routes![user]).get("/?\
hello&\
name=Bob+Smith&\
id=1337&\
active=yes\
# ").dispatch().status();
# assert_eq!(status, rocket::http::Status::Ok);
```

## Error Catchers

Application processing is fallible. Errors arise from the following sources:

  * A failing guard.
  * A failing responder.
  * A routing failure.

If any of these occur, Rocket returns an error to the client. To generate the
error, Rocket invokes the _catcher_ corresponding to the error's status code and
scope. Catchers are similar to routes except in that:

  1. Catchers are only invoked on error conditions.
  2. Catchers are declared with the `catch` attribute.
  3. Catchers are _registered_ with [`register()`] instead of [`mount()`].
  4. Any modifications to cookies are cleared before a catcher is invoked.
  5. Error catchers cannot invoke guards.
  6. Error catchers should not fail to produce a response.
  7. Catchers are scoped to a path prefix.

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
    rocket::build().register("/", catchers![not_found]);
}
```

### Scoping

The first argument to `register()` is a path to scope the catcher under called
the catcher's _base_. A catcher's base determines which requests it will handle
errors for. Specifically, a catcher's base must be a prefix of the erroring
request for it to be invoked. When multiple catchers can be invoked, the catcher
with the longest base takes precedence.

As an example, consider the following application:

```rust
# #[macro_use] extern crate rocket;

#[catch(404)]
fn general_not_found() -> &'static str {
    "General 404"
}

#[catch(404)]
fn foo_not_found() -> &'static str {
    "Foo 404"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .register("/", catchers![general_not_found])
        .register("/foo", catchers![foo_not_found])
}

# let client = rocket::local::blocking::Client::debug(rocket()).unwrap();
#
# let response = client.get("/").dispatch();
# assert_eq!(response.into_string().unwrap(), "General 404");
#
# let response = client.get("/bar").dispatch();
# assert_eq!(response.into_string().unwrap(), "General 404");
#
# let response = client.get("/bar/baz").dispatch();
# assert_eq!(response.into_string().unwrap(), "General 404");
#
# let response = client.get("/foo").dispatch();
# assert_eq!(response.into_string().unwrap(), "Foo 404");
#
# let response = client.get("/foo/bar").dispatch();
# assert_eq!(response.into_string().unwrap(), "Foo 404");
```

Since there are no mounted routes, all requests will `404`. Any request whose
path begins with `/foo` (i.e, `GET /foo`, `GET /foo/bar`, etc) will be handled
by the `foo_not_found` catcher while all other requests will be handled by the
`general_not_found` catcher.

### Default Catchers

A _default_ catcher is a catcher that handles _all_ status codes. They are
invoked as a fallback if no status-specific catcher is registered for a given
error. Declaring a default catcher is done with `#[catch(default)]` and must
similarly be registered with [`register()`]:

```rust
# #[macro_use] extern crate rocket;

use rocket::Request;
use rocket::http::Status;

#[catch(default)]
fn default_catcher(status: Status, request: &Request) { /* .. */ }

#[launch]
fn rocket() -> _ {
    rocket::build().register("/", catchers![default_catcher])
}
```

Catchers with longer bases are preferred, even when there is a status-specific
catcher. In other words, a default catcher with a longer matching base than a
status-specific catcher takes precedence.

### Built-In Catcher

Rocket provides a built-in default catcher. It produces HTML or JSON, depending
on the value of the `Accept` header. As such, custom catchers only need to be
registered for custom error handling.

The [error handling example](@git/master/examples/error-handling) illustrates catcher use in
full, while the [`Catcher`] API documentation provides further details.

[`catch`]: @api/master/rocket/attr.catch.html
[`register()`]: @api/master/rocket/struct.Rocket.html#method.register
[`mount()`]: @api/master/rocket/struct.Rocket.html#method.mount
[`catchers!`]: @api/master/rocket/macro.catchers.html
[`&Request`]: @api/master/rocket/struct.Request.html
[`Status`]: @api/master/rocket/http/struct.Status.html
[`Catcher`]: @api/master/rocket/catcher/struct.Catcher.html
