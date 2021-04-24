use rocket::form::{Form, Strict, FromForm, FromFormField, Errors};

fn strict<'f, T: FromForm<'f>>(string: &'f str) -> Result<T, Errors<'f>> {
    Form::<Strict<T>>::parse(string).map(|s| s.into_inner())
}

fn lenient<'f, T: FromForm<'f>>(string: &'f str) -> Result<T, Errors<'f>> {
    Form::<T>::parse(string)
}

fn strict_encoded<T: 'static>(string: &'static str) -> Result<T, Errors<'static>>
    where for<'a> T: FromForm<'a>
{
    Form::<Strict<T>>::parse_encoded(string.into()).map(|s| s.into_inner())
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

    let task: Option<TodoTask> = lenient("other=a&description=Hello&completed=on").ok();
    assert_eq!(task, Some(TodoTask {
        description: "Hello".to_string(),
        completed: true
    }));

    // Ensure _method isn't required.
    let task: Option<TodoTask> = strict("_method=patch&description=Hello&completed=off").ok();
    assert_eq!(task, Some(TodoTask {
        description: "Hello".to_string(),
        completed: false
    }));
}

#[derive(Debug, PartialEq, FromFormField)]
enum FormOption {
    A, B, C
}

#[derive(Debug, PartialEq, FromForm)]
struct FormInput<'r> {
    checkbox: bool,
    number: usize,
    radio: FormOption,
    password: &'r str,
    textarea: String,
    select: FormOption,
}

#[derive(Debug, PartialEq, FromForm)]
struct DefaultInput<'r> {
    arg: Option<&'r str>,
}

#[derive(Debug, PartialEq, FromForm)]
struct ManualMethod<'r> {
    _method: Option<&'r str>,
    done: bool
}

#[derive(Debug, PartialEq, FromForm)]
struct UnpresentCheckbox {
    checkbox: bool
}

#[derive(Debug, PartialEq, FromForm)]
struct UnpresentCheckboxTwo<'r> {
    checkbox: bool,
    something: &'r str
}

#[derive(Debug, PartialEq, FromForm)]
struct FieldNamedV<'r> {
    v: &'r str,
}

#[test]
fn base_conditions() {
    let form_string = &[
        "password=testing", "checkbox=off", "number=10", "textarea=",
        "select=a", "radio=c",
    ].join("&");

    let input: Result<FormInput<'_>, _> = strict(&form_string);
    assert_eq!(input, Ok(FormInput {
        checkbox: false,
        number: 10,
        radio: FormOption::C,
        password: "testing".into(),
        textarea: "".to_string(),
        select: FormOption::A,
    }));

    // Argument not in string with default in form.
    let default: Option<DefaultInput<'_>> = strict("").ok();
    assert_eq!(default, Some(DefaultInput {
        arg: None
    }));

    // Ensure _method can be captured if desired.
    let manual: Option<ManualMethod<'_>> = strict("_method=put&done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: Some("put".into()),
        done: true
    }));

    let manual: Option<ManualMethod<'_>> = lenient("_method=put&done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: Some("put".into()),
        done: true
    }));

    // And ignored when not present.
    let manual: Option<ManualMethod<'_>> = strict("done=true").ok();
    assert_eq!(manual, Some(ManualMethod {
        _method: None,
        done: true
    }));

    // Check that a `bool` value that isn't in the form is marked as `false`.
    let manual: Option<UnpresentCheckbox> = lenient("").ok();
    assert_eq!(manual, Some(UnpresentCheckbox {
        checkbox: false
    }));

    // Check that a `bool` value that isn't in the form is marked as `false`.
    let manual: Option<UnpresentCheckboxTwo<'_>> = lenient("something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    // Check that a structure with one field `v` parses correctly.
    let manual: Option<FieldNamedV<'_>> = strict("v=abc").ok();
    assert_eq!(manual, Some(FieldNamedV {
        v: "abc".into()
    }));
}

#[test]
fn lenient_parsing() {
    // Check that a structure with one field `v` parses correctly (lenient).
    let manual: Option<FieldNamedV<'_>> = lenient("v=abc").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    let manual: Option<FieldNamedV<'_>> = lenient("v=abc&a=123").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    let manual: Option<FieldNamedV<'_>> = lenient("c=abcddef&v=abc&a=123").ok();
    assert_eq!(manual, Some(FieldNamedV { v: "abc".into() }));

    // Check default values (bool) with lenient parsing.
    let manual: Option<UnpresentCheckboxTwo<'_>> = lenient("something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    let manual: Option<UnpresentCheckboxTwo<'_>> = lenient("hi=hi&something=hello").ok();
    assert_eq!(manual, Some(UnpresentCheckboxTwo {
        checkbox: false,
        something: "hello".into()
    }));

    // Check that a missing field doesn't parse, even leniently.
    let manual: Option<FieldNamedV<'_>> = lenient("a=abc").ok();
    assert!(manual.is_none());

    let manual: Option<FieldNamedV<'_>> = lenient("_method=abc").ok();
    assert!(manual.is_none());
}

#[test]
fn field_renaming() {
    #[derive(Debug, PartialEq, FromForm)]
    struct RenamedForm {
        single: usize,
        #[field(name = "camelCase")]
        camel_case: String,
        #[field(name = "TitleCase")]
        title_case: String,
        #[field(name = "type")]
        field_type: isize,
        #[field(name = "DOUBLE")]
        double: String,
        #[field(name = "a:b")]
        colon: isize,
    }

    let form_string = &[
        "single=100", "camelCase=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "a:b=123"
    ].join("&");

    let form: Option<RenamedForm> = strict(&form_string).ok();
    assert_eq!(form, Some(RenamedForm {
        single: 100,
        camel_case: "helloThere".into(),
        title_case: "HiHi".into(),
        field_type: -2,
        double: "bing_bong".into(),
        colon: 123,
    }));

    let form_string = &[
        "single=100", "camel_case=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "colon=123"
    ].join("&");

    let form: Option<RenamedForm> = strict(&form_string).ok();
    assert!(form.is_none());

    #[derive(Debug, PartialEq, FromForm)]
    struct MultiName<'r> {
        single: usize,
        #[field(name = "SomeCase")]
        #[field(name = "some_case")]
        some_case: &'r str,
    }

    let form_string = &["single=123", "some_case=hi_im_here"].join("&");
    let form: Option<MultiName> = strict(&form_string).ok();
    assert_eq!(form, Some(MultiName { single: 123, some_case: "hi_im_here", }));

    let form_string = &["single=123", "SomeCase=HiImHere"].join("&");
    let form: Option<MultiName> = strict(&form_string).ok();
    assert_eq!(form, Some(MultiName { single: 123, some_case: "HiImHere", }));

    let form_string = &["single=123", "some_case=hi_im_here", "SomeCase=HiImHere"].join("&");
    let form: Option<MultiName> = strict(&form_string).ok();
    assert!(form.is_none());

    let form_string = &["single=123", "some_case=hi_im_here", "SomeCase=HiImHere"].join("&");
    let form: Option<MultiName> = lenient(&form_string).ok();
    assert_eq!(form, Some(MultiName { single: 123, some_case: "hi_im_here", }));

    let form_string = &["single=123", "SomeCase=HiImHere", "some_case=hi_im_here"].join("&");
    let form: Option<MultiName> = lenient(&form_string).ok();
    assert_eq!(form, Some(MultiName { single: 123, some_case: "HiImHere", }));

    #[derive(Debug, PartialEq, FromForm)]
    struct CaseInsensitive<'r> {
        #[field(name = uncased("SomeCase"))]
        #[field(name = "some_case")]
        some_case: &'r str,

        #[field(name = uncased("hello"))]
        hello: usize,
    }

    let form_string = &["HeLLO=123", "sOMECASe=hi_im_here"].join("&");
    let form: Option<CaseInsensitive> = strict(&form_string).ok();
    assert_eq!(form, Some(CaseInsensitive { hello: 123, some_case: "hi_im_here", }));

    let form_string = &["hello=456", "SomeCase=HiImHere"].join("&");
    let form: Option<CaseInsensitive> = strict(&form_string).ok();
    assert_eq!(form, Some(CaseInsensitive { hello: 456, some_case: "HiImHere", }));

    let form_string = &["helLO=789", "some_case=hi_there"].join("&");
    let form: Option<CaseInsensitive> = strict(&form_string).ok();
    assert_eq!(form, Some(CaseInsensitive { hello: 789, some_case: "hi_there", }));

    let form_string = &["hello=123", "SOme_case=hi_im_here"].join("&");
    let form: Option<CaseInsensitive> = strict(&form_string).ok();
    assert!(form.is_none());
}

#[test]
fn generics() {
    #[derive(FromForm, Debug, PartialEq)]
    struct Oops<A, B, C> {
        base: String,
        a: A,
        b: B,
        c: C,
    }

    #[derive(FromForm, Debug, PartialEq)]
    struct YetOneMore<'f, T> {
        string: &'f str,
        other: T,
    }

    let form_string = &[
        "string=hello", "other=00128"
    ].join("&");

    let form: Option<YetOneMore<'_, usize>> = strict(&form_string).ok();
    assert_eq!(form, Some(YetOneMore {
        string: "hello".into(),
        other: 128,
    }));

    let form: Option<YetOneMore<'_, u8>> = strict(&form_string).ok();
    assert_eq!(form, Some(YetOneMore {
        string: "hello".into(),
        other: 128,
    }));

    let form: Option<YetOneMore<'_, i8>> = strict(&form_string).ok();
    assert!(form.is_none());

    let form_string = "base=just%20a%20test&a=hey%20there&b=a&c=811";
    let form: Option<Oops<String, FormOption, usize>> = strict_encoded(&form_string).ok();
    assert_eq!(form, Some(Oops {
        base: "just a test".into(),
        a: "hey there".into(),
        b: FormOption::A,
        c: 811,
    }));
}

#[test]
fn form_errors() {
    use rocket::form::error::{ErrorKind, Entity};

    #[derive(Debug, PartialEq, FromForm)]
    struct WhoopsForm {
        complete: bool,
        other: usize,
    }

    let form: Result<WhoopsForm, _> = strict("complete=true&other=781");
    assert_eq!(form, Ok(WhoopsForm { complete: true, other: 781 }));

    let errors = strict::<WhoopsForm>("complete=true&other=unknown").unwrap_err();
    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "other"
            && e.value.as_deref() == Some("unknown")
            && matches!(e.kind, ErrorKind::Int(..))
    }));

    let errors = strict::<WhoopsForm>("complete=unknown&other=unknown").unwrap_err();
    assert!(errors.iter().any(|e| {
        "complete" == e.name.as_ref().unwrap()
            && e.value.as_deref() == Some("unknown")
            && matches!(e.kind, ErrorKind::Bool(..))
    }));

    let errors = strict::<WhoopsForm>("complete=true&other=1&extra=foo").unwrap_err();
    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "extra"
            && e.value.as_deref() == Some("foo")
            && matches!(e.kind, ErrorKind::Unexpected)
    }));

    let errors = strict::<WhoopsForm>("complete=unknown&unknown=!").unwrap_err();
    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "complete"
            && e.value.as_deref() == Some("unknown")
            && matches!(e.kind, ErrorKind::Bool(..))
    }));

    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "unknown"
            && e.value.as_deref() == Some("!")
            && matches!(e.kind, ErrorKind::Unexpected)
    }));

    let errors = strict::<WhoopsForm>("unknown=!").unwrap_err();
    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "unknown"
            && e.value.as_deref() == Some("!")
            && matches!(e.kind, ErrorKind::Unexpected)
    }));

    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "complete"
            && e.value.is_none()
            && e.entity == Entity::Field
            && matches!(e.kind, ErrorKind::Missing)
    }));

    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "other"
            && e.value.is_none()
            && e.entity == Entity::Field
            && matches!(e.kind, ErrorKind::Missing)
    }));

    let errors = strict::<WhoopsForm>("complete=true").unwrap_err();
    assert!(errors.iter().any(|e| {
        e.name.as_ref().unwrap() == "other"
            && e.value.is_none()
            && e.entity == Entity::Field
            && matches!(e.kind, ErrorKind::Missing)
    }));
}

#[test]
fn raw_ident_form() {
    #[derive(Debug, PartialEq, FromForm)]
    struct RawIdentForm {
        r#type: String,
    }

    let form: Result<RawIdentForm, _> = strict("type=a");
    assert_eq!(form, Ok(RawIdentForm { r#type: "a".into() }));
}

#[test]
fn test_multi() {
    use std::collections::HashMap;

    #[derive(Debug, PartialEq, FromForm)]
    struct Multi<'r> {
        checks: Vec<bool>,
        names: Vec<&'r str>,
        news: Vec<String>,
        dogs: HashMap<String, Dog>,
        #[field(name = "more:dogs")]
        more_dogs: HashMap<&'r str, Dog>,
    }

    let multi: Multi = strict("checks=true&checks=false&checks=false\
        &names=Sam&names[]=Smith&names[]=Bob\
        &news[]=Here&news[]=also here\
        &dogs[fido].barks=true&dogs[George].barks=false\
        &dogs[fido].trained=on&dogs[George].trained=yes\
        &dogs[bob boo].trained=no&dogs[bob boo].barks=off\
        &more:dogs[k:0]=My Dog&more:dogs[v:0].barks=true&more:dogs[v:0].trained=yes\
    ").unwrap();
    assert_eq!(multi, Multi {
        checks: vec![true, false, false],
        names: vec!["Sam".into(), "Smith".into(), "Bob".into()],
        news: vec!["Here".into(), "also here".into()],
        dogs: {
            let mut map = HashMap::new();
            map.insert("fido".into(), Dog { barks: true, trained: true });
            map.insert("George".into(), Dog { barks: false, trained: true });
            map.insert("bob boo".into(), Dog { barks: false, trained: false });
            map
        },
        more_dogs: {
            let mut map = HashMap::new();
            map.insert("My Dog".into(), Dog { barks: true, trained: true });
            map
        }
    });

    #[derive(Debug, PartialEq, FromForm)]
    struct MultiOwned {
        names: Vec<String>,
    }

    let raw = "names=Sam&names%5B%5D=Smith&names%5B%5D=Bob%20Smith%3F";
    let multi: MultiOwned = strict_encoded(raw).unwrap();
    assert_eq!(multi, MultiOwned {
        names: vec!["Sam".into(), "Smith".into(), "Bob Smith?".into()],
    });
}

#[derive(Debug, FromForm, PartialEq)]
struct Dog {
    barks: bool,
    trained: bool,
}

#[derive(Debug, FromForm, PartialEq)]
struct Cat<'r> {
    nip: &'r str,
    meows: bool
}

#[derive(Debug, FromForm, PartialEq)]
struct Pet<'r, T> {
    pet: T,
    name: &'r str,
    age: u8
}

#[derive(Debug, PartialEq, FromForm)]
struct Person<'r> {
    dogs: Vec<Pet<'r, Dog>>,
    cats: Vec<Pet<'r, Cat<'r>>>,
    sitting: Dog,
}

#[test]
fn test_nested_multi() {
    let person: Person = strict("sitting.barks=true&sitting.trained=true").unwrap();
    assert_eq!(person, Person {
        sitting: Dog { barks: true, trained: true },
        cats: vec![],
        dogs: vec![],
    });

    let person: Person = strict("sitting.barks=true&sitting.trained=true\
        &dogs[0].name=fido&dogs[0].pet.trained=yes&dogs[0].age=7&dogs[0].pet.barks=no\
    ").unwrap();
    assert_eq!(person, Person {
        sitting: Dog { barks: true, trained: true },
        cats: vec![],
        dogs: vec![Pet {
            pet: Dog { barks: false, trained: true },
            name: "fido".into(),
            age: 7
        }]
    });

    let person: Person = strict("sitting.trained=no&sitting.barks=true\
        &dogs[0].name=fido&dogs[0].pet.trained=yes&dogs[0].age=7&dogs[0].pet.barks=no\
        &dogs[1].pet.barks=true&dogs[1].name=Bob&dogs[1].pet.trained=no&dogs[1].age=1\
    ").unwrap();
    assert_eq!(person, Person {
        sitting: Dog { barks: true, trained: false },
        cats: vec![],
        dogs: vec![
            Pet {
                pet: Dog { barks: false, trained: true },
                name: "fido".into(),
                age: 7
            },
            Pet {
                pet: Dog { barks: true, trained: false },
                name: "Bob".into(),
                age: 1
            },
        ]
    });

    let person: Person = strict("sitting.barks=true&sitting.trained=no\
        &dogs[0].name=fido&dogs[0].pet.trained=yes&dogs[0].age=7&dogs[0].pet.barks=no\
        &dogs[1].pet.barks=true&dogs[1].name=Bob&dogs[1].pet.trained=no&dogs[1].age=1\
        &cats[george].pet.nip=paws&cats[george].name=George&cats[george].age=2\
        &cats[george].pet.meows=yes\
    ").unwrap();
    assert_eq!(person, Person {
        sitting: Dog { barks: true, trained: false },
        cats: vec![
            Pet {
                pet: Cat { nip: "paws".into(), meows: true },
                name: "George".into(),
                age: 2
            }
        ],
        dogs: vec![
            Pet {
                pet: Dog { barks: false, trained: true },
                name: "fido".into(),
                age: 7
            },
            Pet {
                pet: Dog { barks: true, trained: false },
                name: "Bob".into(),
                age: 1
            },
        ]
    });
}

<<<<<<< HEAD
#[test]
fn test_multipart() {
    use rocket::http::ContentType;
    use rocket::local::blocking::Client;
    use rocket::fs::TempFile;

    #[derive(FromForm)]
    struct MyForm<'r> {
        names: Vec<&'r str>,
        file: TempFile<'r>,
    }

    #[rocket::post("/", data = "<form>")]
    fn form(form: Form<MyForm>) {
        assert_eq!(form.names, &["abcd", "123"]);
        assert_eq!(form.file.name(), Some("foo"));
    }

    let client = Client::debug_with(rocket::routes![form]).unwrap();
    let ct = "multipart/form-data; boundary=X-BOUNDARY"
        .parse::<ContentType>()
        .unwrap();

    let body = &[
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="names[]""#,
        "",
        "abcd",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="names[]""#,
        "",
        "123",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="file"; filename="foo.txt""#,
        "Content-Type: text/plain",
        "",
        "hi there",
        "--X-BOUNDARY--",
        "",
    ].join("\r\n");

    let response = client.post("/")
        .header(ct)
        .body(body)
        .dispatch();

    assert!(response.status().class().is_success());
=======
// fn test_multipart() {
//     use std::{io, path::Path};
//
//     use crate::*;
//     use crate::http::ContentType;
//     use crate::local::blocking::Client;
//     use crate::{data::TempFile, form::Errors};
//
//     #[derive(FromForm)]
//     struct MyForm {
//         names: Vec<String>,
//         file: String,
//     }
//
//     #[post("/", data = "<form>")]
//     async fn form(mut form: Form<MyForm>) -> io::Result<&'static str> {
//         let path = Path::new("/tmp").join(form.file_name().unwrap_or("upload"));
//         form.persist(path).await?;
//         println!("result: {:?}", form);
//         Ok("hi")
//     }
//
//     let client = Client::debug_with(routes![form]).unwrap();
//     let ct = "multipart/form-data; boundary=X-BOUNDARY"
//         .parse::<ContentType>()
//         .unwrap();
//
//     let body = &[
//         // "--X-BOUNDARY",
//         // r#"Content-Disposition: form-data; name="names[]""#,
//         // "",
//         // "abcd",
//         // "--X-BOUNDARY",
//         // r#"Content-Disposition: form-data; name="names[]""#,
//         // "",
//         // "123",
//         "--X-BOUNDARY",
//         r#"Content-Disposition: form-data; name="file"; filename="foo.txt""#,
//         "Content-Type: text/plain",
//         "",
//         "hi there",
//         "--X-BOUNDARY--",
//         "",
//     ].join("\r\n");
//
//     let response = client.post("/")
//         .header(ct)
//         .body(body)
//         .dispatch();
//
//     let string = response.into_string().unwrap();
//     println!("String: {}", string);
//     panic!(string);
// }

#[test]
fn test_defaults() {
    #[derive(FromForm, PartialEq, Debug)]
    struct FormWithDefaults {
        #[field(default = 100)]
        field1: i32,
        field2: i32,

        #[field(default = true)]
        field3: bool,
        #[field(default = false)]
        field4: bool,
        field5: bool,
    }

    let form_string = &["field1=101", "field2=102"].join("&");

    let form: Option<FormWithDefaults> = lenient(&form_string).ok();
    assert_eq!(form, Some(FormWithDefaults {
        field1: 101,
        field2: 102,
        field3: true,
        field4: false,
        field5: false,
    }));
    let form: Option<FormWithDefaults> = strict(&form_string).ok();
    assert!(form.is_none());

>>>>>>> 6681f074 (update grammar, enforce single default, added failing and succeeding tests, update FromForm docs, update FromForm guide, fixed CI errors)
}
