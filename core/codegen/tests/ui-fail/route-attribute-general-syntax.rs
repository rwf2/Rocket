#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

// Check a path is supplied, at least.

#[get()] //~ ERROR missing expected parameter
fn a0() {}

// Check that it only works on functions.

#[get("/")]
struct S;
//~^ ERROR expected `fn`
//~^^ HELP on functions

#[get("/")]
enum A {  }
//~^ ERROR expected `fn`
//~^^ HELP on functions

#[get("/")]
trait Foo {  }
//~^ ERROR expected `fn`
//~^^ HELP on functions

#[get("/")]
impl S {  }
//~^ ERROR expected `fn`
//~^^ HELP on functions

// Check that additional parameter weirdness is caught.

#[get("/", 123)] //~ ERROR expected
fn b0() {}

#[get("/", "/")] //~ ERROR expected
fn b1() {}

#[get(data = "<foo>", "/")] //~ ERROR unexpected keyed parameter
fn b2(foo: usize) {}

#[get("/", unknown = "foo")] //~ ERROR unexpected
fn b3() {}

#[get("/", ...)] //~ ERROR malformed
//~^ HELP expected syntax
fn b4() {}

// Check that all identifiers are named

#[get("/")]
fn c1(_: usize) {} //~ ERROR cannot be ignored
//~^ HELP must be of the form

// Check that the path is a string, rank is an integer.

#[get(100)] //~ ERROR expected string
fn d0() {}

#[get('/')] //~ ERROR expected string
fn d1() {}

#[get("/", rank = "1")] //~ ERROR expected integer
fn d2() {}

#[get("/", rank = '1')] //~ ERROR expected integer
fn d3() {}

// Check that formats are valid media-type strings.

#[get("/", format = "applicationx-custom")] //~ ERROR invalid or unknown media type
fn e0() {}

#[get("/", format = "")] //~ ERROR invalid or unknown media type
fn e1() {}

#[get("/", format = "//")] //~ ERROR invalid or unknown media type
fn e2() {}

#[get("/", format = "/")] //~ ERROR invalid or unknown media type
fn e3() {}

#[get("/", format = "a/")] //~ ERROR invalid or unknown media type
fn e4() {}

#[get("/", format = "/a")] //~ ERROR invalid or unknown media type
fn e5() {}

#[get("/", format = "/a/")] //~ ERROR invalid or unknown media type
fn e6() {}

#[get("/", format = "a/b/")] //~ ERROR invalid or unknown media type
fn e7() {}

#[get("/", format = "unknown")] //~ ERROR unknown media type
fn e8() {}

#[get("/", format = 12)] //~ ERROR expected string
fn e9() {}

#[get("/", format = 'j')] //~ ERROR expected string
fn e10() {}

#[get("/", format = "text//foo")] //~ ERROR invalid or unknown media type
fn e12() {}

// Check that route methods are validated properly.

#[route(CONNECT, "/")] //~ ERROR invalid HTTP method for route
//~^ HELP method must be one of
fn f0() {}

#[route(FIX, "/")] //~ ERROR invalid HTTP method
//~^ HELP method must be one of
fn f1() {}

#[route("hi", "/")] //~ ERROR expected identifier
//~^ HELP method must be one of
fn f2() {}

#[route("GET", "/")] //~ ERROR expected identifier
//~^ HELP method must be one of
fn f3() {}

#[route(120, "/")] //~ ERROR expected identifier
//~^ HELP method must be one of
fn f4() {}

fn main() {}
