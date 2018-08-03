#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

#[get("/<todo>")]
fn todo(todo: String) -> String {
    todo
}

#[test]
fn main() {
    let _ = routes![todo];
}
