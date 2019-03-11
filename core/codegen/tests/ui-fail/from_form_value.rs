#[macro_use] extern crate rocket;

#[derive(FromFormValue)]
struct Foo1;
//~^ ERROR not supported

#[derive(FromFormValue)]
struct Foo2(usize);
//~^ ERROR not supported

#[derive(FromFormValue)]
struct Foo3 {
//~^ ERROR not supported
    foo: usize,
}

#[derive(FromFormValue)]
enum Foo4 {
    A(usize),
    //~^ ERROR cannot have fields
}

#[derive(FromFormValue)]
enum Foo5 { }
//~^ WARNING empty enum

#[derive(FromFormValue)]
enum Foo6<T> {
//~^ ERROR type generics are not supported
    A(T),
}

#[derive(FromFormValue)]
enum Bar1 {
    #[form(value = 123)]
    //~^ ERROR invalid value: expected string
    A,
}

#[derive(FromFormValue)]
enum Bar2 {
    #[form(value)]
    //~^ ERROR expected literal or key/value
    A,
}

fn main() { }
