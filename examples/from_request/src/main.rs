#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::fmt;
use rocket::request::{self, Request, FromRequest};
use rocket::outcome::Outcome::*;

#[derive(Debug)]
struct HeaderCount(usize);

impl fmt::Display for HeaderCount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for HeaderCount {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        Success(HeaderCount(request.headers().len()))
    }
}

#[get("/")]
fn header_count(header_count: HeaderCount) -> String {
    format!("Your request contained {} headers!", header_count)
}

fn main() {
    rocket::ignite().mount("/", routes![header_count]).launch()
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::Header;

    fn test_header_count<'h>(headers: Vec<Header<'static>>) {
        let rocket = rocket::ignite()
            .mount("/", routes![super::header_count]);

        let num_headers = headers.len();
        let mut req = MockRequest::new(Get, "/");
        for header in headers {
            req = req.header(header);
        }

        let mut response = req.dispatch_with(&rocket);

        let expect = format!("Your request contained {} headers!", num_headers);
        assert_eq!(response.body().and_then(|b| b.into_string()), Some(expect));
    }

    #[test]
    fn test_n_headers() {
        for i in 0..50 {
            let mut headers = vec![];
            for j in 0..i {
                let string = format!("{}", j);
                headers.push(Header::new(string.clone(), string));
            }

            test_header_count(headers);
        }
    }
}
