use rocket::http::Header;
use rocket::local::blocking::Client;

#[test]
fn test_local_request_clone_soundness() {
    let client = Client::tracked(rocket::ignite()).unwrap();

    // creates two LocalRequest instances that shouldn't share the same req
    let r1 = client.get("/").header(Header::new("key", "val1"));
    let mut r2 = r1.clone();

    // save the iterator, which internally holds a slice
    let mut iter = r1.inner().headers().get("key");

    // insert headers to force header map reallocation.
    for i in 0..100 {
        r2.add_header(Header::new(i.to_string(), i.to_string()));
    }

    // Replace the original key/val.
    r2.add_header(Header::new("key", "val2"));

    // Heap massage: so we've got crud to print.
    let _: Vec<usize> = vec![0, 0xcafebabe, 31337, 0];

    // Ensure we're good.
    let s = iter.next().unwrap();
    println!("{}", s);

    // And that we've got the right data.
    assert_eq!(r1.inner().headers().get("key").collect::<Vec<_>>(), vec!["val1"]);
    assert_eq!(r2.inner().headers().get("key").collect::<Vec<_>>(), vec!["val1", "val2"]);
}
