#![feature(plugin)]
#![plugin(rocket_codegen)]
#![allow(dead_code, unused_variables)]
#![deny(unmounted_route, unmanaged_state)]

extern crate rocket;

use rocket::State;

type MyState<'r> = State<'r, usize>;

type MyVecState<'r, T: 'r> = State<'r, Vec<T>>;

#[get("/")]
fn index(state: MyState) {  }

#[get("/a")]
fn another(state: MyVecState<usize>) {  }

fn main() {
    rocket::ignite()
        .manage(10usize)
        .manage(vec![1usize, 2usize, 3usize])
        .mount("/", routes![index, another]);
}
