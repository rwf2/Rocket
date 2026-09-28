+++
summary = "a migration guide from Rocket v0.5 to v0.6"
+++

# Upgrading

This a placeholder for an eventual migration guide from v0.5 to v0.6.

## Mount Points

Mount points, or bases, are now treated like route URIs: they are never
percent-decoded and are instead compared against the percent-decoded request
path. Previously, a base had to be a valid origin URI.

  * [`Rocket::mount()`], [`Rocket::register()`], [`Route::rebase()`], and
    [`Catcher::rebase()`] take a `&str` base. Pass a `String` as `&base` and
    a URI as `uri.path().as_str()`.
  * A base may contain any UTF-8 characters. A route mounted at `/häcker`
    matches requests to `/h%C3%A4cker/..`.
  * A `%` in a catcher base is no longer a percent-encoding escape. A catcher
    registered at `/a%20b` now catches errors from requests to `/a%2520b/..`,
    where routes mounted at `/a%20b` have always matched, and not `/a%20b/..`.
  * `uri!` percent-encodes static segments of a route URI and literal prefixes
    starting with `/`. These prefixes use mount-point syntax: a prefix of
    `/a%20b` generates `/a%2520b`. To use an already encoded path, pass a URI
    expression: `uri!(uri!("/a%20b"), handler)`. Absolute URI prefixes also
    preserve percent-encoding escapes.

[`Rocket::mount()`]: @api/master/rocket/struct.Rocket.html#method.mount
[`Rocket::register()`]: @api/master/rocket/struct.Rocket.html#method.register
[`Route::rebase()`]: @api/master/rocket/struct.Route.html#method.rebase
[`Catcher::rebase()`]: @api/master/rocket/struct.Catcher.html#method.rebase

## Getting Help

If you run into any issues upgrading, we encourage you to ask questions via
[GitHub discussions] or via chat at [`#rocket:mozilla.org`] on Matrix. The
[FAQ](../faq/) also provides answers to commonly asked questions.

[GitHub discussions]: @github/discussions
[`#rocket:mozilla.org`]: @chat
