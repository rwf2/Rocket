#[macro_use] extern crate rocket;

#[derive(FromFormValue)]
struct Foo1;

#[derive(FromFormValue)]
struct Foo2(usize);

#[derive(FromFormValue)]
struct Foo3 {
    foo: usize,
}

#[derive(FromFormValue)]
enum Foo4 {
    A(usize),
}

#[derive(FromFormValue)]
enum Foo5 { }

#[derive(FromFormValue)]
enum Foo6<T> {
    A(T),
}

#[derive(FromFormValue)]
enum Bar1 {
    #[form(value = 123)]
    A,
}

#[derive(FromFormValue)]
enum Bar2 {
    #[form(value)]
    A,
}

fn main() { }
