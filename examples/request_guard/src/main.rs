#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::request::{self, Request, FromRequest};
use rocket::outcome::Outcome::*;

#[derive(Debug)]
struct HeaderCount(usize);

impl<'a, 'r> FromRequest<'a, 'r> for HeaderCount {
    type Error = std::convert::Infallible;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        Success(HeaderCount(request.headers().len()))
    }
}

#[get("/")]
fn header_count(header_count: HeaderCount) -> String {
    format!("Your request contained {} headers!", header_count.0)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![header_count])
}

fn main() {
    rocket().launch();
}

#[cfg(test)]
mod test {
    use rocket::local::Client;
    use rocket::http::Header;

    fn test_header_count<'h>(headers: Vec<Header<'static>>) {
        let client = Client::new(super::rocket()).unwrap();
        let mut req = client.get("/");
        for header in headers.iter().cloned() {
            req.add_header(header);
        }

        let mut response = req.dispatch();
        let expect = format!("Your request contained {} headers!", headers.len());
        assert_eq!(response.body_string_wait(), Some(expect));
    }

    #[test]
    fn test_n_headers() {
        for i in 0..50 {
            let headers = (0..i)
                .map(|n| Header::new(n.to_string(), n.to_string()))
                .collect();

            test_header_count(headers);
        }
    }
}
