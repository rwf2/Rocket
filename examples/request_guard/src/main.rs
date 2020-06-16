#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::request::{self, Request, FromRequest};
use rocket::outcome::Outcome::*;

#[derive(Debug)]
struct HeaderCount(usize);

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for HeaderCount {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        Success(HeaderCount(request.headers().len()))
    }
}

#[get("/")]
fn header_count(header_count: HeaderCount) -> String {
    format!("Your request contained {} headers!", header_count.0)
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![header_count])
}

#[cfg(test)]
mod test {
    use rocket::local::Client;
    use rocket::http::Header;

    async fn test_header_count<'h>(headers: Vec<Header<'static>>) {
        let client = Client::new(super::rocket()).await.unwrap();
        let mut req = client.get("/");
        for header in headers.iter().cloned() {
            req.add_header(header);
        }

        let mut response = req.dispatch().await;
        let expect = format!("Your request contained {} headers!", headers.len());
        assert_eq!(response.body_string().await, Some(expect));
    }

    #[rocket::async_test]
    async fn test_n_headers() {
        for i in 0..50 {
            let headers = (0..i)
                .map(|n| Header::new(n.to_string(), n.to_string()))
                .collect();

            test_header_count(headers).await;
        }
    }
}
