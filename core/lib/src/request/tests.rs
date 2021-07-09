use std::collections::HashMap;

use crate::Request;
use crate::local::blocking::Client;
use crate::http::hyper;

macro_rules! assert_headers {
    ($($key:expr => [$($value:expr),+]),+) => ({
        // Create a new Hyper request. Add all of the passed in headers.
        let mut req = hyper::Request::get("/test").body(()).unwrap();
        $($(req.headers_mut().append($key, hyper::HeaderValue::from_str($value).unwrap());)+)+

        // Build up what we expect the headers to actually be.
        let mut expected = HashMap::new();
        $(expected.entry($key).or_insert(vec![]).append(&mut vec![$($value),+]);)+

        // Create a valid `Rocket` and convert the hyper req to a Rocket one.
        let client = Client::debug_with(vec![]).unwrap();
        let hyper = req.into_parts().0;
        let req = Request::from_hyp(client.rocket(), &hyper, None).unwrap();

        // Dispatch the request and check that the headers match.
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
