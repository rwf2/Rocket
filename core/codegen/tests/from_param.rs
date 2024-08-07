use rocket::request::FromParam;

#[derive(Debug, FromParam, PartialEq)]
enum Test {
    Test1,
    Test2
}

#[test]
fn derive_from_param() {
    let test1 = Test::from_param("Test1").expect("Should be valid");
    assert_eq!(test1, Test::Test1);

    let test2 = Test::from_param("Test2").expect("Should be valid");
    assert_eq!(test2, Test::Test2);

    let test3 = Test::from_param("not_test");
    assert!(test3.is_err());
}
