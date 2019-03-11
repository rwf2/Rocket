#[macro_use] extern crate rocket;

use rocket::http::RawStr;

#[derive(FromForm)]
enum Thing { }
//~^ ERROR not supported

#[derive(FromForm)]
struct Foo1;
//~^ ERROR not supported

#[derive(FromForm)]
struct Foo2 {  }
//~^ ERROR one field is required

#[derive(FromForm)]
struct Foo3(usize);
//~^ ERROR not supported

#[derive(FromForm)]
struct NextTodoTask<'f, 'a> {
//~^ ERROR only one lifetime
    description: String,
    raw_description: &'f RawStr,
    other: &'a RawStr,
    completed: bool,
}

#[derive(FromForm)]
struct BadName1 {
    #[form(field = "isindex")]
    //~^ ERROR invalid form field name
    field: String,
}

#[derive(FromForm)]
struct Demo2 {
    #[form(field = "foo")]
    field: String,
    foo: usize,
    //~^ ERROR duplicate field
}

#[derive(FromForm)]
struct MyForm9 {
    #[form(field = "hello")]
    first: String,
    #[form(field = "hello")]
    //~^ ERROR duplicate field
    other: String,
}

#[derive(FromForm)]
struct MyForm10 {
    first: String,
    #[form(field = "first")]
    //~^ ERROR duplicate field
    other: String,
}

#[derive(FromForm)]
struct MyForm {
    #[form(field = "blah", field = "bloo")]
    //~^ ERROR duplicate
    my_field: String,
}

#[derive(FromForm)]
struct MyForm1 {
    #[form]
    //~^ ERROR malformed attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm2 {
    #[form("blah")]
    //~^ ERROR expected key/value
    my_field: String,
}

#[derive(FromForm)]
struct MyForm3 {
    #[form(123)]
    //~^ ERROR expected key/value
    my_field: String,
}

#[derive(FromForm)]
struct MyForm4 {
    #[form(beep = "bop")]
    //~^ ERROR unexpected attribute parameter
    my_field: String,
}

#[derive(FromForm)]
struct MyForm5 {
    #[form(field = "blah")]
    #[form(field = "bleh")]
    //~^ ERROR duplicate
    my_field: String,
}

#[derive(FromForm)]
struct MyForm6 {
    #[form(field = true)]
    //~^ ERROR invalid value: expected string
    my_field: String,
}

#[derive(FromForm)]
struct MyForm7 {
    #[form(field)]
    //~^ ERROR expected literal or key/value
    my_field: String,
}

#[derive(FromForm)]
struct MyForm8 {
    #[form(field = 123)]
    //~^ ERROR invalid value: expected string
    my_field: String,
}

#[derive(FromForm)]
struct MyForm11 {
    #[form(field = "hello&world")]
    //~^ ERROR invalid form field name
    first: String,
}

#[derive(FromForm)]
struct MyForm12 {
    #[form(field = "!@#$%^&*()_")]
    //~^ ERROR invalid form field name
    first: String,
}

#[derive(FromForm)]
struct MyForm13 {
    #[form(field = "?")]
    //~^ ERROR invalid form field name
    first: String,
}

#[derive(FromForm)]
struct MyForm14 {
    #[form(field = "")]
    //~^ ERROR invalid form field name
    first: String,
}

#[derive(FromForm)]
struct BadName2 {
    #[form(field = "a&b")]
    //~^ ERROR invalid form field name
    field: String,
}

#[derive(FromForm)]
struct BadName3 {
    #[form(field = "a=")]
    //~^ ERROR invalid form field name
    field: String,
}

fn main() { }
