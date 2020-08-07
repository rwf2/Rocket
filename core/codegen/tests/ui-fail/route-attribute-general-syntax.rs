#[macro_use] extern crate rocket;

// Check a path is supplied, at least.
#[get()]
fn a0() {}

// Check that it only works on functions.
#[get("/")]
struct S;

#[get("/")]
enum A {  }

#[get("/")]
trait Foo {  }

#[get("/")]
impl S {  }

// Check that additional parameter weirdness is caught.
#[get("/", 123)]
fn b0() {}

#[get("/", "/")]
fn b1() {}

#[get(data = "<foo>", "/")]
fn b2(foo: usize) {}

#[get("/", unknown = "foo")]
fn b3() {}

#[get("/", ...)]
fn b4() {}

// Check that all identifiers are named

#[get("/")]
fn c1(_: usize) {}

// Check that the path is a string, rank is an integer.

#[get(100)]
fn d0() {}

#[get('/')]
fn d1() {}

#[get("/", rank = "1")]
fn d2() {}

#[get("/", rank = '1')]
fn d3() {}

// Check that formats are valid media-type strings.

#[get("/", format = "applicationx-custom")]
fn e0() {}

#[get("/", format = "")]
fn e1() {}

#[get("/", format = "//")]
fn e2() {}

#[get("/", format = "/")]
fn e3() {}

#[get("/", format = "a/")]
fn e4() {}

#[get("/", format = "/a")]
fn e5() {}

#[get("/", format = "/a/")]
fn e6() {}

#[get("/", format = "a/b/")]
fn e7() {}

#[get("/", format = "unknown")]
fn e8() {}

#[get("/", format = 12)]
fn e9() {}

#[get("/", format = 'j')]
fn e10() {}

#[get("/", format = "text//foo")]
fn e12() {}

// Check that route methods are validated properly.

#[route(CONNECT, "/")]
fn f0() {}

#[route(FIX, "/")]
fn f1() {}

#[route("hi", "/")]
fn f2() {}

#[route("GET", "/")]
fn f3() {}

#[route(120, "/")]
fn f4() {}

fn main() {}
