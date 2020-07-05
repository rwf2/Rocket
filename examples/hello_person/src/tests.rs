use rocket::local::blocking::Client;
use rocket::http::Status;

fn test(uri: String, expected: String) {
    let client = Client::new(super::rocket()).unwrap();
    assert_eq!(client.get(&uri).dispatch().into_string(), Some(expected));
}

fn test_404(uri: &'static str) {
    let client = Client::new(super::rocket()).unwrap();
    assert_eq!(client.get(uri).dispatch().status(), Status::NotFound);
}

#[test]
fn test_hello() {
    for &(name, age) in &[("Mike", 22), ("Michael", 80), ("A", 0), ("a", 127)] {
        test(format!("/hello/{}/{}", name, age),
            format!("Hello, {} year old named {}!", age, name));
    }
}

#[test]
fn test_failing_hello() {
    test_404("/hello/Mike/1000");
    test_404("/hello/Mike/-129");
    test_404("/hello/Mike/-1");
}

#[test]
fn test_hi() {
    for name in &["Mike", "A", "123", "hi", "c"] {
        test(format!("/hello/{}", name), name.to_string());
    }
}
