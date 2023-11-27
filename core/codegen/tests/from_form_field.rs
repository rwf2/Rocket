use rocket::form::{Errors, FromForm, FromFormField, Options, ValueField};

fn parse<'v, T: FromForm<'v>>(value: &'v str) -> Result<T, Errors<'v>> {
    let mut context = T::init(Options::Lenient);
    T::push_value(&mut context, ValueField::from_value(value));
    T::finalize(context)
}

macro_rules! assert_parse {
    ($($string:expr),* => $item:ident :: $variant:ident) => ($(
        match parse::<$item>($string) {
            Ok($item::$variant) => { /* okay */ },
            Ok(item) => panic!("Failed to parse {} as {:?}. Got {:?} instead.",
                               $string, $item::$variant, item),
            Err(e) => panic!("Failed to parse {} as {}: {}",
                             $string, stringify!($item), e),

        }
    )*)
}

macro_rules! assert_no_parse {
    ($($string:expr),* => $item:ident) => ($(
        match parse::<$item>($string) {
            Err(_) => { /* okay */ },
            Ok(item) => panic!("Unexpectedly parsed {} as {:?}", $string, item)
        }
    )*)
}

#[test]
fn from_form_value_simple() {
    #[derive(Debug, FromFormField)]
    enum Foo {
        A,
        B,
        C,
    }

    assert_parse!("a", "A" => Foo::A);
    assert_parse!("b", "B" => Foo::B);
    assert_parse!("c", "C" => Foo::C);
}

#[test]
fn from_form_value_weirder() {
    #[allow(non_camel_case_types)]
    #[derive(Debug, FromFormField)]
    enum Foo {
        Ab_Cd,
        OtherA,
    }

    assert_parse!("ab_cd", "ab_CD", "Ab_CD" => Foo::Ab_Cd);
    assert_parse!("othera", "OTHERA", "otherA", "OtherA" => Foo::OtherA);
}

#[test]
fn from_form_value_no_parse() {
    #[derive(Debug, FromFormField)]
    enum Foo {
        A,
        B,
        C,
    }

    assert_no_parse!("abc", "ab", "bc", "ca" => Foo);
    assert_no_parse!("b ", "a ", "c ", "a b" => Foo);
}

#[test]
fn from_form_value_renames() {
    #[derive(Debug, FromFormField)]
    enum Foo {
        #[field(value = "foo")]
        #[field(value = "bark")]
        Bar,
        #[field(value = ":book")]
        Book,
    }

    assert_parse!("foo", "FOO", "FoO", "bark", "BARK", "BaRk" => Foo::Bar);
    assert_parse!(":book", ":BOOK", ":bOOk", ":booK" => Foo::Book);
    assert_no_parse!("book", "bar" => Foo);
}

#[test]
fn from_form_value_raw() {
    #[allow(non_camel_case_types)]
    #[derive(Debug, FromFormField)]
    enum Keyword {
        r#type,
        this,
    }

    assert_parse!("type", "tYpE" => Keyword::r#type);
    assert_parse!("this" => Keyword::this);
    assert_no_parse!("r#type" => Keyword);
}

#[test]
fn form_value_errors() {
    use rocket::form::error::{Entity, ErrorKind};

    #[derive(Debug, FromFormField)]
    enum Foo {
        Bar,
        Bob,
    }

    let errors = parse::<Foo>("blob").unwrap_err();
    assert!(errors.iter().any(|e| {
        &&"blob" == &e.value.as_ref().unwrap()
            && e.entity == Entity::Value
            && match &e.kind {
                ErrorKind::InvalidChoice { choices } => &choices[..] == &["Bar", "Bob"],
                _ => false,
            }
    }));
}
