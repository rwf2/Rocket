#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[post("/<id>/<name>")]
fn simple(id: i32, name: String) -> &'static str { "" }

fn main() {
    uri!(simple: id = 100, "Hello"); //~ ERROR named and unnamed
    uri!(simple: "Hello", id = 100); //~ ERROR named and unnamed
    uri!(simple,); //~ ERROR expected `:`
    uri!(simple:); //~ ERROR argument list
    uri!("/mount"); //~ ERROR route path
    uri!("/mount",); //~ ERROR expected identifier
    uri!("mount", simple); //~ invalid mount point
    uri!("/mount/<id>", simple); //~ invalid mount point
    uri!(); //~ unexpected end of input
    uri!(simple: id = ); //~ expected expression
}
