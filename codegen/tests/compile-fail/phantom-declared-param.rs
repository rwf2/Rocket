#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("/<param>")] //~ ERROR unused dynamic parameter: `param`
fn get() {  } //~ NOTE expected

#[get("/<a>")] //~ ERROR unused dynamic parameter: `a`
fn get2() {  } //~ NOTE expected

#[get("/a/b/c/<a>/<b>")]
    //~^ ERROR unused dynamic parameter: `a`
    //~^^ ERROR unused dynamic parameter: `b`
fn get32() {  }
    //~^ NOTE expected
    //~^^ NOTE expected

fn main() {  }
