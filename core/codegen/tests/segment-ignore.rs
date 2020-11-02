#[macro_use]
extern crate rocket;

#[get("/<_>", rank = 1)] fn ig_1() -> &'static str { "1" }

#[get("/static")] fn just_static() -> &'static str { "static" }

#[get("/<_>/<_>", rank = 1)] fn ig_2() -> &'static str { "2" }

#[get("/static/<_>")] fn ig_1_static() -> &'static str { "static_1" }

#[get("/<_>/<_>/<_>", rank = 1)] fn ig_3() -> &'static str { "3" }

#[get("/static/<_>/static")] fn ig_1_static_static() -> &'static str { "static_1_static" }

#[get("/<a>/<_>/<_>/<b>")] fn wrapped(a: String, b: String) -> String { a + &b }

#[test]
fn test_ignored_segments() {
    use rocket::local::blocking::Client;

    fn get_string(client: &Client, url: &str) -> String {
        client.get(url).dispatch().into_string().unwrap()
    }

    let rocket = rocket::ignite().mount("/", routes![
        ig_1, just_static, ig_2, ig_3, ig_1_static, ig_1_static_static, wrapped
    ]);

    let client = match Client::untracked(rocket) {
        Ok(client) => client,
        Err(e) => { drop(e); panic!("whoops") },
    };

    assert_eq!(get_string(&client, "/foo"), "1");
    assert_eq!(get_string(&client, "/bar"), "1");
    assert_eq!(get_string(&client, "/static"), "static");

    assert_eq!(get_string(&client, "/foo/bar"), "2");
    assert_eq!(get_string(&client, "/bar/foo"), "2");
    assert_eq!(get_string(&client, "/a/b"), "2");
    assert_eq!(get_string(&client, "/foo/static"), "2");
    assert_eq!(get_string(&client, "/static/foo"), "static_1");

    assert_eq!(get_string(&client, "/foo/bar/baz"), "3");
    assert_eq!(get_string(&client, "/bar/static/bam"), "3");
    assert_eq!(get_string(&client, "/static/static/static"), "static_1_static");
    assert_eq!(get_string(&client, "/static/foo/bam"), "3");

    assert_eq!(get_string(&client, "/a/b/c/d"), "ad");
    assert_eq!(get_string(&client, "/static/b/c/static"), "staticstatic");
    assert_eq!(get_string(&client, "/a/b/c/static"), "astatic");
    assert_eq!(get_string(&client, "/ec/b/c/static"), "ecstatic");
}
