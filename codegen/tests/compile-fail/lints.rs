#![feature(plugin)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]
#![deny(unmanaged_state)]

extern crate rocket;

use rocket::State;

struct MyType;
struct MySecondType;

mod external {
    #[get("/state/extern")]
    fn unmanaged(_c: ::State<i32>) {  }
    //~^ ERROR not currently being managed

    #[get("/state/extern")]
    fn managed(_c: ::State<u32>) {  }
    //~^ WARN is not mounted

    #[get("/state/extern")]
    fn unmanaged_unmounted(_c: ::State<u8>) {  }
    //~^ WARN is not mounted
    //~^^ WARN is not mounted
}

#[get("/state/bad")]
fn unmanaged(_b: State<MySecondType>) {  }
//~^ ERROR not currently being managed
//~^^ ERROR not currently being managed

#[get("/state/ok")]
fn managed(_a: State<u32>) {  }

#[get("/state/bad")]
fn managed_two(_b: State<MyType>) {  }

#[get("/state/ok")]
fn unmounted_doesnt_error(_a: State<i8>) {  }
//~^ WARN is not mounted
//~^^ WARN is not mounted

#[get("/ignored")]
#[allow(unmanaged_state)]
fn ignored(_b: State<u16>) {  }
//~^ WARN is not mounted

#[get("/unmounted/ignored")]
#[allow(unmounted_route)]
fn unmounted_ignored() {  }

#[get("/mounted/nonce")]
fn mounted_only_once() {  }
//~^ WARN is not mounted

fn main() {
    rocket::ignite()
        .mount("/", routes![managed, unmanaged, external::unmanaged])
        .mount("/", routes![managed_two, ignored, mounted_only_once])
        .manage(MyType)
        .manage(100u32);

    rocket::ignite()
        .mount("/", routes![managed, unmanaged, external::unmanaged])
        .mount("/", routes![external::managed, managed_two])
        .manage(MyType)
        .manage(100i32)
        .manage(100u32);
}
