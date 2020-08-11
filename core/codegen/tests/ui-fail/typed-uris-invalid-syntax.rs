#[macro_use] extern crate rocket;

#[post("/<id>/<name>")]
fn simple(id: i32, name: String) -> &'static str { "" }

fn main() {
    uri!(simple: id = 100, "Hello");
    uri!(simple: "Hello", id = 100);
    uri!(simple,);
    uri!(simple:);
    uri!("/mount");
    uri!("/mount",);
    uri!("mount", simple);
    uri!("/mount/<id>", simple);
    uri!();
    uri!(simple: id = );
}
