#[macro_use] extern crate rocket;

use rocket::http::CookieJar;

#[post("/<id>")]
fn has_one(id: i32) {  }

#[post("/<id>")]
fn has_one_guarded(cookies: &CookieJar<'_>, id: i32) {  }

#[post("/<id>?<name>")]
fn has_two(cookies: &CookieJar<'_>, id: i32, name: String) {  }

#[post("/<id>/<name>")]
fn optionals(id: Option<i32>, name: Result<String, &str>) {  }

#[post("/<_>")]
fn ignored() {  }

fn main() {
    uri!(has_one);
    uri!(has_one());

    uri!(has_one(1, 23));
    uri!(has_one("Hello", 23, ));
    uri!(has_one_guarded("hi", 100));

    uri!(has_two(10, "hi", "there"));
    uri!(has_two(10));

    uri!(has_one(id = 100, name = "hi"));

    uri!(has_one(name = 100, id = 100));

    uri!(has_one(name = 100, age = 50, id = 100));

    uri!(has_one(name = 100, age = 50, id = 100, id = 50));

    uri!(has_one(id = 100, id = 100));

    uri!(has_one(id = 100, id = 100, ));

    uri!(has_one(name = "hi"));

    uri!(has_one_guarded(cookies = "hi", id = 100));

    uri!(has_one_guarded(id = 100, cookies = "hi"));

    uri!(has_two(id = 100, id = 100, ));

    uri!(has_two(name = "hi"));

    uri!(has_two(cookies = "hi", id = 100, id = 10, id = 10));

    uri!(has_two(id = 100, cookies = "hi"));

    uri!(optionals(id = _, name = "bob".into()));

    uri!(optionals(id = 10, name = _));

    uri!(ignored(_));

    uri!(ignored(_ = 10));

    uri!(ignored(10, 20));

    uri!(ignored(num = 10));

    uri!(ignored(10, "10"));
}
