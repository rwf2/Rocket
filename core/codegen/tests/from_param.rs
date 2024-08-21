use rocket::request::FromParam;

#[allow(non_camel_case_types)]
#[derive(Debug, FromParam, PartialEq)]
enum Test {
    Test1,
    Test2,
    r#for,
}

#[test]
fn derive_from_param() {
    assert_eq!(Test::from_param("Test1").unwrap(), Test::Test1);
    assert_eq!(Test::from_param("Test2").unwrap(), Test::Test2);
    assert_eq!(Test::from_param("for").unwrap(), Test::r#for);

    let err = Test::from_param("For").unwrap_err();
    assert_eq!(err.value, "For");
    assert_eq!(err.options, &["Test1", "Test2", "for"]);

    let err = Test::from_param("not_test").unwrap_err();
    assert_eq!(err.value, "not_test");
    assert_eq!(err.options, &["Test1", "Test2", "for"]);

}
