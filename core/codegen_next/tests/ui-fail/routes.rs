#![feature(plugin, decl_macro, proc_macro_non_items)]

#[macro_use] extern crate rocket;

fn main() {
    let _ = routes![a b];
    //~^ ERROR expected
}
