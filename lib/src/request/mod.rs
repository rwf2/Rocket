//! Types and traits for request parsing and handling.

mod request;
mod param;
mod form;
mod from_request;
mod state;

pub use self::request::Request;
pub use self::from_request::{FromRequest, Outcome};
pub use self::param::{FromParam, FromSegments};
pub use self::form::{Form, FromForm, FromFormValue, FormItems};
pub use self::state::State;

/// Type alias to retrieve [Flash](/rocket/response/struct.Flash.html) messages
/// from a request.
pub type FlashMessage = ::response::Flash<()>;

#[cfg(test)]
mod tests {
    /// These tests are related to Issue#223
    /// The way we were getting the headers from hyper
    /// was causing a list to come back as a comma separated
    /// list of entries.

    use super::Request;
    use super::super::http::hyper::header::Headers;
    use hyper::method::Method;
    use hyper::uri::RequestUri;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::collections::HashMap;

    fn check_headers(test_headers: HashMap<String, Vec<String>>) {
        let h_method: Method = Method::Get;
        let h_uri: RequestUri = RequestUri::AbsolutePath("/test".to_string());
        let h_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let mut h_headers: Headers = Headers::new();

        for (key, values) in &test_headers {
            let raw_bytes: Vec<Vec<u8>> = values
                .iter()
                .map(|v| v.clone().into_bytes())
                .collect();
            h_headers.set_raw(key.clone(), raw_bytes);
        }

        let req = match Request::from_hyp(h_method, h_headers, h_uri, h_addr) {
            Ok(req) => req,
            Err(e) => panic!("Building Request failed: {:?}", e),
        };

        let r_headers = req.headers();

        for (key, values) in &test_headers {
            for (v1, v2) in values.iter().zip(r_headers.get(&key)) {
                assert_eq!(v1, v2)
            }
        }
    }

    #[test]
    fn test_single_header_single_entry() {
        let mut test_headers = HashMap::new();
        test_headers.insert("friends".to_string(), vec![
            "alice".to_string(),
        ]);
        check_headers(test_headers);
    }

    #[test]
    fn test_single_header_multiple_entries() {
        let mut test_headers = HashMap::new();
        test_headers.insert("friends".to_string(), vec![
            "alice".to_string(),
            "bob".to_string()
        ]);
        check_headers(test_headers);
    }

    #[test]
    fn test_single_header_comma_entry() {
        let mut test_headers = HashMap::new();
        test_headers.insert("friends".to_string(), vec![
            "alice".to_string(),
            "bob, carol".to_string()
        ]);
        check_headers(test_headers);
    }

    #[test]
    fn test_multiple_headers_single_entry() {
        let mut test_headers = HashMap::new();
        test_headers.insert("friends".to_string(), vec![
            "alice".to_string(),
        ]);
        test_headers.insert("enemies".to_string(), vec![
            "victor".to_string(),
        ]);
        check_headers(test_headers);
    }

    #[test]
    fn test_multiple_headers_multiple_entries() {
        let mut test_headers = HashMap::new();
        test_headers.insert("friends".to_string(), vec![
            "alice".to_string(),
            "bob".to_string(),
        ]);
        test_headers.insert("enemies".to_string(), vec![
            "david".to_string(),
            "emily".to_string(),
        ]);
        check_headers(test_headers);
    }
}
