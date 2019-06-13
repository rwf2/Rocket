use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;

use crate::{Rocket, Request, Config};
use crate::http::hyper;

macro_rules! assert_headers {
    ($($key:expr => [$($value:expr),+]),+) => ({
        // Set up the parameters to the hyper request object.
        let h_method = hyper::Method::Get;
        let h_uri = hyper::RequestUri::AbsolutePath("/test".to_string());
        let h_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let mut h_headers = hyper::header::Headers::new();

        // Add all of the passed in headers to the request.
        $($(h_headers.append_raw($key.to_string(), $value.as_bytes().into());)+)+

        // Build up what we expect the headers to actually be.
        let mut expected = HashMap::new();
        $(expected.entry($key).or_insert(vec![]).append(&mut vec![$($value),+]);)+

        // Dispatch the request and check that the headers are what we expect.
        let config = Config::development();
        let r = Rocket::custom(config);
        let req = Request::from_hyp(&r, h_method, h_headers, h_uri, h_addr).unwrap();
        let actual_headers = req.headers();
        for (key, values) in expected.iter() {
            let actual: Vec<_> = actual_headers.get(key).collect();
            assert_eq!(*values, actual);
        }
    })
}

#[test]
fn test_multiple_headers_from_hyp() {
    assert_headers!("friends" => ["alice"]);
    assert_headers!("friends" => ["alice", "bob"]);
    assert_headers!("friends" => ["alice", "bob, carol"]);
    assert_headers!("friends" => ["alice, david", "bob, carol", "eric, frank"]);
    assert_headers!("friends" => ["alice"], "enemies" => ["victor"]);
    assert_headers!("friends" => ["alice", "bob"], "enemies" => ["david", "emily"]);
}

#[test]
fn test_multiple_headers_merge_into_one_from_hyp() {
    assert_headers!("friend" => ["alice"], "friend" => ["bob"]);
    assert_headers!("friend" => ["alice"], "friend" => ["bob"], "friend" => ["carol"]);
    assert_headers!("friend" => ["alice"], "friend" => ["bob"], "enemy" => ["carol"]);
}
