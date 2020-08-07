#[macro_use] extern crate rocket;

// Check that route paths are absolute and normalized.

#[get("a")]
fn f0() {}

#[get("")]
fn f1() {}

#[get("a/b/c")]
fn f2() {}

#[get("/a///b")]
fn f3() {}

#[get("/?bat&&")]
fn f4() {}

#[get("/?bat&&")]
fn f5() {}

#[get("/a/b//")]
fn f6() {}

// Check that paths contain only valid URI characters

#[get("/!@#$%^&*()")]
fn g1() {}

#[get("/a%20b")]
fn g2() {}

#[get("/a?a%20b")]
fn g3() {}

#[get("/a?a+b")]
fn g4() {}

// Check that all declared parameters are accounted for

#[get("/<name>")]
fn h0(_name: usize) {}

#[get("/a?<r>")]
fn h1() {}

#[post("/a", data = "<test>")]
fn h2() {}

#[get("/<_r>")]
fn h3() {}

#[get("/<_r>/<b>")]
fn h4() {}


// Check dynamic parameters are valid idents

#[get("/<foo_.>")]
fn i0() {}

#[get("/<foo*>")]
fn i1() {}

#[get("/<!>")]
fn i2() {}

#[get("/<name>:<id>")]
fn i3() {}

// Check that a data parameter is exactly `<param>`

#[get("/", data = "foo")]
fn j0() {}

#[get("/", data = "<foo..>")]
fn j1() {}

#[get("/", data = "<foo")]
fn j2() {}

#[get("/", data = "<test >")]
fn j3() {}

// Check that all identifiers are named

#[get("/<_>")]
fn k0(_: usize) {}

// Check that strange dynamic syntax is caught.

#[get("/<>")]
fn m0() {}

#[get("/<id><")]
fn m1() {}

#[get("/<<<<id><")]
fn m2() {}

#[get("/<>name><")]
fn m3() {}

fn main() {  }
