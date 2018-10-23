// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use] extern crate rocket;

// #[get("/", rank = 1)]
// fn get1() -> &'static str { "hi" }

// #[get("/", rank = 2)]
// fn get2() -> &'static str { "hi" }

// #[get("/", rank = 3)]
// fn get3() -> &'static str { "hi" }

// #[get("/")] fn get() {  }
// #[route(GET, "/")] fn get_r() {  }

// #[put("/")] fn put() {  }
// #[route(PUT, "/")] fn put_r() {  }

// #[post("/")] fn post() {  }
// #[route(POST, "/")] fn post_r() {  }

// #[delete("/")] fn delete() {  }
// #[route(DELETE, "/")] fn delete_r() {  }

// #[head("/")] fn head() {  }
// #[route(HEAD, "/")] fn head_r() {  }

// #[patch("/")] fn patch() {  }
// #[route(PATCH, "/")] fn patch_r() {  }

// #[options("/")] fn options() {  }
// #[route(OPTIONS, "/")] fn options_r() {  }

// use rocket::http::{Cookies, RawStr};
// use rocket::request::Form;

// #[derive(FromForm)]
// struct User<'a> {
//     name: &'a RawStr,
//     nickname: String,
// }

// #[post("/<_name>?<_query>", format = "application/json", data = "<user>", rank = 2)]
// fn get(
//     _name: &RawStr,
//     _query: User,
//     user: Form<User>,
//     _cookies: Cookies
// ) -> String {
//     format!("{}:{}", user.name, user.nickname)
// }

// #[post("/", format = "application/x-custom")]
// fn get() -> &'static str { "hi" }

// #[get("/test/<one>/<two>/<three>")]
// fn get(one: String, two: usize, three: isize) -> &'static str { "hi" }

// #[get("/test/<_one>/<_two>/<__three>")]
// fn ignored(_one: String, _two: usize, __three: isize) -> &'static str { "hi" }

// #[get("/")]
// fn get() -> &'static str { "hi" }

// #[get("/")]
// fn get_empty() {  }

// #[get("/one")]
// fn one() {  }

// #[get("/two")]
// fn two() {  }

// #[get("/three")]
// fn three() {  }

// #[get("/four")]
// fn four() {  }

// #[test]
// fn main() {
//     let instance = rocket::ignite()
//         .mount("/", routes![one]);

//     let other = instance.mount("/", routes![two]);
//     other.mount("/", routes![three])
//         .mount("/", routes![four]);

//     rocket::ignite()
//         .mount("/", routes![one])
//         .mount("/", routes![two])
//         .mount("/", routes![three])
//         .mount("/", routes![four]);

//     let a = rocket::ignite()
//         .mount("/", routes![one])
//         .mount("/", routes![two]);

//     let b = a.mount("/", routes![three])
//         .mount("/", routes![four]);
// }

// #[get("/<todo>")]
// fn todo(todo: String) -> String {
//     todo
// }

// #[post("/<a>/<b..>")]
// fn get(a: String, b: PathBuf) -> String {
//     format!("{}/{}", a, b.to_string_lossy())
// }

// #[post("/<a>/<b..>")]
// fn get2(a: String, b: Result<PathBuf, SegmentError>) -> String {
//     format!("{}/{}", a, b.unwrap().to_string_lossy())
// }


// #[test]
// fn main() {
//     let _ = routes![todo];
// }
