#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

// Check that route paths are absolute and normalized.

#[get("a")] //~ ERROR invalid path URI
//~^ HELP expected
fn f0() {}

#[get("")] //~ ERROR invalid path URI
//~^ HELP expected
fn f1() {}

#[get("a/b/c")] //~ ERROR invalid path URI
//~^ HELP expected
fn f2() {}

#[get("/a///b")] //~ ERROR empty segments
//~^ NOTE expected
fn f3() {}

#[get("/?bat&&")] //~ ERROR empty segments
fn f4() {}

#[get("/?bat&&")] //~ ERROR empty segments
fn f5() {}

#[get("/a/b//")] //~ ERROR empty segments
//~^ NOTE expected
fn f6() {}

// Check that paths contain only valid URI characters

#[get("/!@#$%^&*()")] //~ ERROR invalid path URI
//~^ HELP origin form
fn g1() {}

#[get("/a%20b")] //~ ERROR invalid URI characters
//~^ NOTE cannot contain reserved
//~^^ HELP reserved characters include
fn g2() {}

#[get("/a?a%20b")] //~ ERROR invalid URI characters
//~^ NOTE cannot contain reserved
//~^^ HELP reserved characters include
fn g3() {}

#[get("/a?a+b")] //~ ERROR invalid URI characters
//~^ NOTE cannot contain reserved
//~^^ HELP reserved characters include
fn g4() {}

// Check that all declared parameters are accounted for

#[get("/<name>")] //~ ERROR unused dynamic parameter
fn h0(_name: usize) {} //~ NOTE expected argument named `name` here

#[get("/a?<r>")] //~ ERROR unused dynamic parameter
fn h1() {} //~ NOTE expected argument named `r` here

#[post("/a", data = "<test>")] //~ ERROR unused dynamic parameter
fn h2() {} //~ NOTE expected argument named `test` here

#[get("/<_r>")] //~ ERROR unused dynamic parameter
fn h3() {} //~ NOTE expected argument named `_r` here

#[get("/<_r>/<b>")] //~ ERROR unused dynamic parameter
//~^ ERROR unused dynamic parameter
fn h4() {} //~ NOTE expected argument named `_r` here
//~^ NOTE expected argument named `b` here

// Check dynamic parameters are valid idents

#[get("/<foo_.>")] //~ ERROR `foo_.` is not a valid identifier
//~^ HELP must be valid
fn i0() {}

#[get("/<foo*>")] //~ ERROR `foo*` is not a valid identifier
//~^ HELP must be valid
fn i1() {}

#[get("/<!>")] //~ ERROR `!` is not a valid identifier
//~^ HELP must be valid
fn i2() {}

#[get("/<name>:<id>")] //~ ERROR `name>:<id` is not a valid identifier
//~^ HELP must be valid
fn i3() {}

// Check that a data parameter is exactly `<param>`

#[get("/", data = "foo")] //~ ERROR malformed parameter
//~^ HELP must be of the form
fn j0() {}

#[get("/", data = "<foo..>")] //~ ERROR malformed parameter
//~^ HELP must be of the form
fn j1() {}

#[get("/", data = "<foo")] //~ ERROR missing a closing bracket
//~^ HELP did you mean
fn j2() {}

#[get("/", data = "<test >")] //~ ERROR `test ` is not a valid identifier
//~^ HELP must be valid
fn j3() {}

// Check that all identifiers are named

#[get("/<_>")] //~ ERROR must be named
fn k0(_: usize) {} //~^ HELP use a name such as

// Check that strange dynamic syntax is caught.

#[get("/<>")] //~ ERROR cannot be empty
fn m0() {}

#[get("/<id><")] //~ ERROR malformed parameter
//~^ HELP must be of the form
//~^^ HELP identifiers cannot contain
fn m1() {}

#[get("/<<<<id><")] //~ ERROR malformed parameter
//~^ HELP must be of the form
//~^^ HELP identifiers cannot contain
fn m2() {}

#[get("/<>name><")] //~ ERROR malformed parameter
//~^ HELP must be of the form
//~^^ HELP identifiers cannot contain
fn m3() {}

fn main() {  }
