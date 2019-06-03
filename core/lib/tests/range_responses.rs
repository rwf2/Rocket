#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[get("/")]
fn data() -> &'static [u8] {
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
}

mod tests {
    use super::*;

    use rocket::Rocket;
    use rocket::local::Client;
    use rocket::http::Status;
    use rocket::http::hyper::header::{Range, ByteRangeSpec};

    fn rocket() -> Rocket {
        rocket::ignite()
            .mount("/", routes![data])
    }

    #[test]
    fn head() {
        let client = Client::new(rocket()).unwrap();
        let response = client.head("/").dispatch();
        let headers = response.headers();

        assert_eq!(headers.get_one("Accept-Ranges"), Some("bytes"));
    }

    #[test]
    fn range_between() {
        let client = Client::new(rocket()).unwrap();
        let mut response = client.get("/")
            .header(Range::bytes(1, 4))
            .dispatch();

        assert_eq!(response.status(), Status::PartialContent);

        {
            let headers = response.headers();
            assert_eq!(headers.get_one("Content-Range"), Some("bytes 1-4/10"));
        }

        assert_eq!(response.body_bytes(), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn range_between_invalid() {
        let client = Client::new(rocket()).unwrap();
        let response = client.get("/")
            .header(Range::bytes(4, 2))
            .dispatch();

        assert_eq!(response.status(), Status::RangeNotSatisfiable);
    }

    #[test]
    fn range_between_overflow() {
        let client = Client::new(rocket()).unwrap();
        let response = client.get("/")
            .header(Range::bytes(11, 12))
            .dispatch();

        assert_eq!(response.status(), Status::RangeNotSatisfiable);
    }

    #[test]
    fn range_from() {
        let client = Client::new(rocket()).unwrap();
        let mut response = client.get("/")
            .header(Range::Bytes(vec![
                ByteRangeSpec::AllFrom(4),
            ]))
            .dispatch();

        assert_eq!(response.status(), Status::PartialContent);

        {
            let headers = response.headers();
            assert_eq!(headers.get_one("Content-Range"), Some("bytes 4-9/10"));
        }

        assert_eq!(response.body_bytes(), Some(vec![4, 5, 6, 7, 8, 9]));
    }

    #[test]
    fn range_from_overflow() {
        let client = Client::new(rocket()).unwrap();
        let response = client.get("/")
            .header(Range::Bytes(vec![
                ByteRangeSpec::AllFrom(12),
            ]))
            .dispatch();

        assert_eq!(response.status(), Status::RangeNotSatisfiable);
    }

    #[test]
    fn range_last() {
        let client = Client::new(rocket()).unwrap();
        let mut response = client.get("/")
            .header(Range::Bytes(vec![
                ByteRangeSpec::Last(3),
            ]))
            .dispatch();

        assert_eq!(response.status(), Status::PartialContent);

        {
            let headers = response.headers();
            assert_eq!(headers.get_one("Content-Range"), Some("bytes 7-9/10"));
        }

        assert_eq!(response.body_bytes(), Some(vec![7, 8, 9]));
    }

    #[test]
    fn range_last_overflow() {
        let client = Client::new(rocket()).unwrap();
        let mut response = client.get("/")
            .header(Range::Bytes(vec![
                ByteRangeSpec::Last(12),
            ]))
            .dispatch();

        assert_eq!(response.status(), Status::PartialContent);

        {
            let headers = response.headers();
            assert_eq!(headers.get_one("Content-Range"), Some("bytes 0-9/10"));
        }

        assert_eq!(response.body_bytes(), Some(Vec::from(data())));
    }
}