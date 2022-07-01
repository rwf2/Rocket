#[macro_use] extern crate rocket;

#[get("/")]
fn index() {  }

#[post("/<id>/<name>")]
fn simple(id: i32, name: String) -> &'static str { "" }

fn main() {
    uri!(simple: id = 100, "Hello");
    uri!(simple(id = 100, "Hello"));
    uri!(simple("Hello", id = 100));
    uri!(simple,);
    uri!(simple:);
    uri!("/mount",);
    uri!("mount", simple);
    uri!("mount", simple, "http://");
    uri!("/mount", simple, "http://");
    uri!("/mount", simple, "#foo", "?foo");
    uri!("mount", simple(10, "hi"), "http://");
    uri!("/mount", simple(10, "hi"), "http://");
    uri!("/mount?foo", simple(10, "hi"), "foo/bar?foo#bar");
    uri!("/mount", simple(10, "hi"), "a/b");
    uri!("/mount", simple(10, "hi"), "#foo", "?foo");
    uri!("/mount/<id>", simple);
    uri!();
    uri!(simple: id = );
    uri!(simple(id = ));
    uri!("*", simple(10), "hi");
    uri!("some.host:8088", simple(10), "hi");
    uri!("?foo");
    uri!("");
    uri!("/foo", "bar");
    uri!("/foo" ("bar"));
    uri!("ftp:?", index);
    uri!("ftp:", index, "foo#bar");
    uri!("ftp:", index, "foo?bar");
}
