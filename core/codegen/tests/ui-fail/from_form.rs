#[macro_use] extern crate rocket;

use rocket::http::RawStr;

#[derive(FromForm)]
enum Thing { }

#[derive(FromForm)]
struct Foo1;

#[derive(FromForm)]
struct Foo2 {  }

#[derive(FromForm)]
struct Foo3(usize);

#[derive(FromForm)]
struct NextTodoTask<'f, 'a> {
    description: String,
    raw_description: &'f RawStr,
    other: &'a RawStr,
    completed: bool,
}

#[derive(FromForm)]
struct BadName1 {
    #[form(field = "isindex")]
    field: String,
}

#[derive(FromForm)]
struct Demo2 {
    #[form(field = "foo")]
    field: String,
    foo: usize,
}

#[derive(FromForm)]
struct MyForm9 {
    #[form(field = "hello")]
    first: String,
    #[form(field = "hello")]
    other: String,
}

#[derive(FromForm)]
struct MyForm10 {
    first: String,
    #[form(field = "first")]
    other: String,
}

#[derive(FromForm)]
struct MyForm {
    #[form(field = "blah", field = "bloo")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm1 {
    #[form]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm2 {
    #[form("blah")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm3 {
    #[form(123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm4 {
    #[form(beep = "bop")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm5 {
    #[form(field = "blah")]
    #[form(field = "bleh")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm6 {
    #[form(field = true)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm7 {
    #[form(field)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm8 {
    #[form(field = 123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm11 {
    #[form(field = "hello&world")]
    first: String,
}

#[derive(FromForm)]
struct MyForm12 {
    #[form(field = "!@#$%^&*()_")]
    first: String,
}

#[derive(FromForm)]
struct MyForm13 {
    #[form(field = "?")]
    first: String,
}

#[derive(FromForm)]
struct MyForm14 {
    #[form(field = "")]
    first: String,
}

#[derive(FromForm)]
struct BadName2 {
    #[form(field = "a&b")]
    field: String,
}

#[derive(FromForm)]
struct BadName3 {
    #[form(field = "a=")]
    field: String,
}

fn main() { }
