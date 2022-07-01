use rocket::form::FromForm;

#[derive(FromForm)]
enum Thing { }

#[derive(FromForm)]
struct Foo1;

#[derive(FromForm)]
struct Foo2 {  }

#[derive(FromForm)]
struct Foo3(usize);

#[derive(FromForm)]
struct Foo4(usize, usize, usize);

#[derive(FromForm)]
struct NextTodoTask<'f, 'a> {
    description: String,
    raw_description: &'f str,
    other: &'a str,
    completed: bool,
}

#[derive(FromForm)]
struct BadName1 {
    #[field(name = "isindex")]
    field: String,
}

#[derive(FromForm)]
struct Demo2 {
    #[field(name = "foo")]
    field: String,
    foo: usize,
}

#[derive(FromForm)]
struct MyForm9 {
    #[field(name = "hello")]
    first: String,
    #[field(name = "hello")]
    other: String,
}

#[derive(FromForm)]
struct MyForm10 {
    first: String,
    #[field(name = "first")]
    other: String,
}

#[derive(FromForm)]
struct MyForm {
    #[field(name = "blah", field = "bloo")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm1 {
    #[field]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm2 {
    #[field("blah")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm3 {
    #[field(123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm4 {
    #[field(beep = "bop")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm5 {
    #[field(name = "blah")]
    #[field(name = "blah")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm6 {
    #[field(name = true)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm7 {
    #[field(name)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm8 {
    #[field(name = 123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm11 {
    #[field(name = "hello&world")]
    first: String,
}

#[derive(FromForm)]
struct MyForm12 {
    #[field(name = "!@#$%^&*()_")]
    first: String,
}

#[derive(FromForm)]
struct MyForm13 {
    #[field(name = "?")]
    first: String,
}

#[derive(FromForm)]
struct MyForm14 {
    #[field(name = "")]
    first: String,
}

#[derive(FromForm)]
struct BadName2 {
    #[field(name = "a&b")]
    field: String,
}

#[derive(FromForm)]
struct BadName3 {
    #[field(name = "a=")]
    field: String,
}

#[derive(FromForm)]
struct Validate0 {
    #[field(validate = 123)]
    first: String,
}

#[derive(FromForm)]
struct Validate1 {
    #[field(validate = unknown())]
    first: String,
}

#[derive(FromForm)]
struct Validate2 {
    #[field(validate = ext(rocket::http::ContentType::HTML))]
    first: String,
}

#[derive(FromForm)]
struct Validate3 {
    #[field(validate = ext("hello"))]
    first: String,
}

#[derive(FromForm)]
struct Default0 {
    #[field(default = 123)]
    first: String,
}

#[derive(FromForm)]
struct Default1 {
    #[field(default = 1, default = 2)]
    double_default: usize,
}

#[derive(FromForm)]
struct Default2 {
    #[field(default = 1)]
    #[field(default = 2)]
    double_default: usize,
}

#[derive(FromForm)]
struct Default3 {
    #[field(default = 1, default_with = None)]
    double_default: usize,
}

#[derive(FromForm)]
struct Default4 {
    #[field(default_with = None)]
    #[field(default = 1)]
    double_default: usize,
}

#[derive(FromForm)]
struct Default5 {
    #[field(default_with = Some("hi"))]
    no_conversion_from_with: String,
}

#[derive(FromForm)]
struct Default6 {
    #[field(default = "no conversion")]
    first: bool,
}

#[derive(FromForm)] // NO ERROR
struct Another<T> {
    _foo: T,
    _bar: T,
}

#[derive(FromForm)] // NO ERROR
struct AnotherOne<T> { // NO ERROR
    _foo: T,
    _bar: T,
}

fn main() { }
