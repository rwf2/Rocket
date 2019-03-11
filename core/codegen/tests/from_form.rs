#[macro_use] extern crate rocket;

use rocket::request::{FromForm, FormItems, FormParseError};
use rocket::http::RawStr;

fn parse<'f, T>(string: &'f str, strict: bool) -> Result<T, FormParseError<'f>>
    where T: FromForm<'f, Error = FormParseError<'f>>
{
    let mut items = FormItems::from(string);
    let result = T::from_form(items.by_ref(), strict);
    if !items.exhaust() {
        panic!("Invalid form input.");
    }

    result
}

fn strict<'f, T>(string: &'f str) -> Result<T, FormParseError<'f>>
    where T: FromForm<'f, Error = FormParseError<'f>>
{
    parse(string, true)
}

fn lenient<'f, T>(string: &'f str) -> Result<T, FormParseError<'f>>
    where T: FromForm<'f, Error = FormParseError<'f>>
{
    parse(string, false)
}

#[derive(Debug, PartialEq, FromForm)]
struct TodoTask {
    description: String,
    completed: bool
}

#[test]
fn simple() {
    // Same number of arguments: simple case.
    let task: Option<TodoTask> = strict("description=Hello&completed=on").ok();
    assert_eq!(task, Some(TodoTask {
        description: "Hello".to_string(),
        completed: true
    }));

    // Argument in string but not in form.
    let task: Option<TodoTask> = strict("other=a&description=Hello&completed=on").ok();
    assert!(task.is_none());

    // Ensure _method isn't required.
    let task: Option<TodoTask> = strict("_method=patch&description=Hello&completed=off").ok();
    assert_eq!(task, Some(TodoTask {
        description: "Hello".to_string(),
        completed: false
    }));
}

#[derive(Debug, PartialEq, FromFormValue)]
enum FormOption {
    A, B, C
}

#[derive(Debug, PartialEq, FromForm)]
struct FormInput<'r> {
    checkbox: bool,
    number: usize,
    radio: FormOption,
    password: &'r RawStr,
    textarea: String,
    select: FormOption,
}

#[derive(Debug, PartialEq, FromForm)]
struct DefaultInput<'r> {
    arg: Option<&'r RawStr>,
}

#[derive(Debug, PartialEq, FromForm)]
struct ManualMethod<'r> {
    _method: Option<&'r RawStr>,
    done: bool
}

#[derive(Debug, PartialEq, FromForm)]
struct UnpresentCheckbox {
    checkbox: bool
}

#[derive(Debug, PartialEq, FromForm)]
struct UnpresentCheckboxTwo<'r> {
    checkbox: bool,
    something: &'r RawStr
}

#[derive(Debug, PartialEq, FromForm)]
struct FieldNamedV<'r> {
    v: &'r RawStr,
}

#[test]
fn base_conditions() {
    let form_string = &[
        "password=testing", "checkbox=off", "checkbox=on", "number=10",
        "checkbox=off", "textarea=", "select=a", "radio=c",
    ].join("&");

    let input: Option<FormInput> = strict(&form_string).ok();
    assert_eq!(input, Some(FormInput {
        checkbox: false,
        number: 10,
        radio: FormOption::C,
        password: "testing".into(),
        textarea: "".to_string(),
        select: FormOption::A,
    }));

    // Argument not in string with default in form.
    let default: Option<DefaultInput> = strict("").ok();
    assert_eq!(default, Some(DefaultInput {
        arg: None
    }));

    // Ensure _method can be captured if desired.
    let manual: Option<ManualMethod> = strict("_method=put&done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: Some("put".into()),
        done: true
    }));

    let manual: Option<ManualMethod> = lenient("_method=put&done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: Some("put".into()),
        done: true
    }));

    // And ignored when not present.
    let manual: Option<ManualMethod> = strict("done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: None,
        done: true
    }));

    // Check that a `bool` value that isn't in the form is marked as `false`.
    let manual: Option<UnpresentCheckbox> = strict("").ok();
    assert_eq!(manual, Some(UnpresentCheckbox {
        checkbox: false
    }));

    // Check that a `bool` value that isn't in the form is marked as `false`.
    let manual: Option<UnpresentCheckboxTwo> = strict("something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    // Check that a structure with one field `v` parses correctly.
    let manual: Option<FieldNamedV> = strict("v=abc").ok();
    assert_eq!(manual, Some(FieldNamedV {
        v: "abc".into()
    }));

}

#[test]
fn lenient_parsing() {
    // Check that a structure with one field `v` parses correctly (lenient).
    let manual: Option<FieldNamedV> = lenient("v=abc").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    let manual: Option<FieldNamedV> = lenient("v=abc&a=123").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    let manual: Option<FieldNamedV> = lenient("c=abcddef&v=abc&a=123").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    // Check default values (bool) with lenient parsing.
    let manual: Option<UnpresentCheckboxTwo> = lenient("something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    let manual: Option<UnpresentCheckboxTwo> = lenient("hi=hi&something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    // Check that a missing field doesn't parse, even leniently.
    let manual: Option<FieldNamedV> = lenient("a=abc").ok();
    assert!(manual.is_none());

    let manual: Option<FieldNamedV> = lenient("_method=abc").ok();
    assert!(manual.is_none());
}

#[derive(Debug, PartialEq, FromForm)]
struct RenamedForm {
    single: usize,
    #[form(field = "camelCase")]
    camel_case: String,
    #[form(field = "TitleCase")]
    title_case: String,
    #[form(field = "type")]
    field_type: isize,
    #[form(field = "DOUBLE")]
    double: String,
    #[form(field = "a.b")]
    dot: isize,
    #[form(field = "some space")]
    some_space: String,
}

#[test]
fn field_renaming() {
    let form_string = &[
        "single=100", "camelCase=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "a.b=123", "some space=okay"
    ].join("&");

    let form: Option<RenamedForm> = strict(&form_string).ok();
    assert_eq!(form, Some(RenamedForm {
        single: 100,
        camel_case: "helloThere".into(),
        title_case: "HiHi".into(),
        field_type: -2,
        double: "bing_bong".into(),
        dot: 123,
        some_space: "okay".into(),
    }));

    let form_string = &[
        "single=100", "camel_case=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "dot=123", "some_space=okay"
    ].join("&");

    let form: Option<RenamedForm> = strict(&form_string).ok();
    assert!(form.is_none());
}

#[derive(FromForm, Debug, PartialEq)]
struct YetOneMore<'f, T> {
    string: &'f RawStr,
    other: T,
}

#[derive(FromForm, Debug, PartialEq)]
struct Oops<A, B, C> {
    base: String,
    a: A,
    b: B,
    c: C,
}

#[test]
fn generics() {
    let form_string = &[
        "string=hello", "other=00128"
    ].join("&");

    let form: Option<YetOneMore<usize>> = strict(&form_string).ok();
    assert_eq!(form, Some(YetOneMore {
        string: "hello".into(),
        other: 128,
    }));

    let form: Option<YetOneMore<u8>> = strict(&form_string).ok();
    assert_eq!(form, Some(YetOneMore {
        string: "hello".into(),
        other: 128,
    }));

    let form: Option<YetOneMore<i8>> = strict(&form_string).ok();
    assert!(form.is_none());

    let form_string = &[
        "base=just%20a%20test", "a=hey%20there", "b=a", "c=811",
    ].join("&");

    let form: Option<Oops<&RawStr, FormOption, usize>> = strict(&form_string).ok();
    assert_eq!(form, Some(Oops {
        base: "just a test".into(),
        a: "hey%20there".into(),
        b: FormOption::A,
        c: 811,
    }));
}

#[derive(Debug, PartialEq, FromForm)]
struct WhoopsForm {
    complete: bool,
    other: usize,
}

#[test]
fn form_errors() {
    let form: Result<WhoopsForm, _> = strict("complete=true&other=781");
    assert_eq!(form, Ok(WhoopsForm { complete: true, other: 781 }));

    let form: Result<WhoopsForm, _> = strict("complete=true&other=unknown");
    assert_eq!(form, Err(FormParseError::BadValue("other".into(), "unknown".into())));

    let form: Result<WhoopsForm, _> = strict("complete=unknown&other=unknown");
    assert_eq!(form, Err(FormParseError::BadValue("complete".into(), "unknown".into())));

    let form: Result<WhoopsForm, _> = strict("complete=true&other=1&extra=foo");
    assert_eq!(form, Err(FormParseError::Unknown("extra".into(), "foo".into())));

    // Bad values take highest precedence.
    let form: Result<WhoopsForm, _> = strict("complete=unknown&unknown=foo");
    assert_eq!(form, Err(FormParseError::BadValue("complete".into(), "unknown".into())));

    // Then unknown key/values for strict parses.
    let form: Result<WhoopsForm, _> = strict("complete=true&unknown=foo");
    assert_eq!(form, Err(FormParseError::Unknown("unknown".into(), "foo".into())));

    // Finally, missing.
    let form: Result<WhoopsForm, _> = strict("complete=true");
    assert_eq!(form, Err(FormParseError::Missing("other".into())));
}
