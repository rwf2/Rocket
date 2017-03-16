# Error Catchers

When Rocket wants to return an error page to the client, Rocket invokes the
catcher for that error. A catcher is like a route, except it only handles
errors. Catchers are declared via the error attribute, which takes a single
integer corresponding to the HTTP status code to catch. For instance, to
declare a catcher for 404 errors, you’d write:

```rust
#[error(404)]
fn not_found(req: &Request) -> String { }
```

As with routes, Rocket needs to know about a catcher before it is used to
handle errors. The process is similar to mounting: call the `catch` method
with a list of catchers via the `errors!` macro. The invocation to add the
404 catcher declared above looks like this:

```rust
rocket::ignite().catch(errors![not_found])
```

Unlike request handlers, error handlers can only take 0, 1, or 2 parameters
of types Request and/or Error. At present, the `Error` type is not
particularly useful, and so it is often omitted. The error catcher example on
 GitHub illustrates their use in full.

Rocket has a default catcher for all of the standard HTTP error codes
including 404, 500, and more.