#[macro_use] extern crate rocket;

#[get("/")]
fn index() {}

fn main() {
    uri!("/foo///");
    uri!("/foo?x&&y");
    uri!("https://example.com//foo");
    uri!("https://example.com?x&&y");
    uri!("foo//bar#f");
    uri!("?x&&y#f");

    uri!("/foo//bar", index);
    uri!("http:////", index);
    uri!("https://example.com//foo", index);

    uri!(_, index, "?x&&y");
    uri!(_, index, "?x&&y#f");
    uri!("/foo", index, "?x&");
}
