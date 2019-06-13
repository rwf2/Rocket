use rocket::request::FromFormValue;

macro_rules! assert_parse {
    ($($string:expr),* => $item:ident :: $variant:ident) => ($(
        match $item::from_form_value($string.into()) {
            Ok($item::$variant) => { /* okay */ },
            Ok(item) => panic!("Failed to parse {} as {:?}. Got {:?} instead.",
                               $string, $item::$variant, item),
            Err(e) => panic!("Failed to parse {} as {}: {:?}",
                             $string, stringify!($item), e),

        }
    )*)
}

macro_rules! assert_no_parse {
    ($($string:expr),* => $item:ident) => ($(
        match $item::from_form_value($string.into()) {
            Err(_) => { /* okay */ },
            Ok(item) => panic!("Unexpectedly parsed {} as {:?}", $string, item)
        }
    )*)
}

#[test]
fn from_form_value_simple() {
    #[derive(Debug, FromFormValue)]
    enum Foo { A, B, C, }

    assert_parse!("a", "A" => Foo::A);
    assert_parse!("b", "B" => Foo::B);
    assert_parse!("c", "C" => Foo::C);
}

#[test]
fn from_form_value_weirder() {
    #[allow(non_camel_case_types)]
    #[derive(Debug, FromFormValue)]
    enum Foo { Ab_Cd, OtherA }

    assert_parse!("ab_cd", "ab_CD", "Ab_CD" => Foo::Ab_Cd);
    assert_parse!("othera", "OTHERA", "otherA", "OtherA" => Foo::OtherA);
}

#[test]
fn from_form_value_no_parse() {
    #[derive(Debug, FromFormValue)]
    enum Foo { A, B, C, }

    assert_no_parse!("abc", "ab", "bc", "ca" => Foo);
    assert_no_parse!("b ", "a ", "c ", "a b" => Foo);
}

#[test]
fn from_form_value_renames() {
    #[derive(Debug, FromFormValue)]
    enum Foo {
        #[form(value = "foo")]
        Bar,
        #[form(value = ":book")]
        Book
    }

    assert_parse!("foo", "FOO", "FoO" => Foo::Bar);
    assert_parse!(":book", ":BOOK", ":bOOk", ":booK" => Foo::Book);
    assert_no_parse!("book", "bar" => Foo);
}
