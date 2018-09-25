#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/easy/<id>")]
fn easy(id: i32) -> String {
    format!("id: {}", id)
}

macro_rules! make_handler {
    () => {
        #[get("/hard/<id>")]
        fn hard(id: i32) -> String {
            format!("id: {}", id)
        }
    }
}

make_handler!();

fn main() { }
