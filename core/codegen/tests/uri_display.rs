#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::http::RawStr;
use rocket::http::uri::{UriDisplay, Query, Path};

macro_rules! assert_uri_display_query {
    ($v:expr, $s:expr) => (
        let uri_string = format!("{}", &$v as &dyn UriDisplay<Query>);
        assert_eq!(uri_string, $s);
    )
}

#[derive(UriDisplayQuery, Clone)]
enum Foo<'r> {
    First(&'r RawStr),
    Second {
        inner: &'r RawStr,
        other: usize,
    },
    Third {
        #[form(field = "type")]
        kind: String,
    },
}

#[test]
fn uri_display_foo() {
    let foo = Foo::First("hello".into());
    assert_uri_display_query!(foo, "hello");

    let foo = Foo::First("hello there".into());
    assert_uri_display_query!(foo, "hello%20there");

    let foo = Foo::Second { inner: "hi".into(), other: 123 };
    assert_uri_display_query!(foo, "inner=hi&other=123");

    let foo = Foo::Second { inner: "hi bo".into(), other: 321 };
    assert_uri_display_query!(foo, "inner=hi%20bo&other=321");

    let foo = Foo::Third { kind: "hello".into() };
    assert_uri_display_query!(foo, "type=hello");

    let foo = Foo::Third { kind: "hello there".into() };
    assert_uri_display_query!(foo, "type=hello%20there");
}

#[derive(UriDisplayQuery)]
struct Bar<'a> {
    foo: Foo<'a>,
    baz: String,
}

#[test]
fn uri_display_bar() {
    let foo = Foo::First("hello".into());
    let bar = Bar { foo, baz: "well, hi!".into() };
    assert_uri_display_query!(bar, "foo=hello&baz=well,%20hi!");

    let foo = Foo::Second { inner: "hi".into(), other: 123 };
    let bar = Bar { foo, baz: "done".into() };
    assert_uri_display_query!(bar, "foo.inner=hi&foo.other=123&baz=done");

    let foo = Foo::Third { kind: "hello".into() };
    let bar = Bar { foo, baz: "turkey day".into() };
    assert_uri_display_query!(bar, "foo.type=hello&baz=turkey%20day");
}

#[derive(UriDisplayQuery)]
struct Baz<'a> {
    foo: Foo<'a>,
    bar: Bar<'a>,
    last: String
}

#[test]
fn uri_display_baz() {
    let foo1 = Foo::Second { inner: "hi".into(), other: 123 };
    let foo2 = Foo::Second { inner: "bye".into(), other: 321 };
    let bar = Bar { foo: foo2, baz: "done".into() };
    let baz = Baz { foo: foo1, bar, last: "ok".into() };
    assert_uri_display_query!(baz, "foo.inner=hi&foo.other=123&\
                              bar.foo.inner=bye&bar.foo.other=321&bar.baz=done&\
                              last=ok");

    let foo1 = Foo::Third { kind: "hello".into() };
    let foo2 = Foo::First("bye".into());
    let bar = Bar { foo: foo1, baz: "end".into() };
    let baz = Baz { foo: foo2, bar, last: "done".into() };
    assert_uri_display_query!(baz, "foo=bye&\
                              bar.foo.type=hello&bar.baz=end&\
                              last=done");
}

#[derive(UriDisplayQuery)]
struct Bam<'a> {
    foo: &'a str,
    bar: Option<usize>,
    baz: Result<&'a RawStr, usize>,
}

#[test]
fn uri_display_bam() {
    let bam = Bam { foo: "hi hi", bar: Some(1), baz: Err(2) };
    assert_uri_display_query!(bam, "foo=hi%20hi&bar=1");

    let bam = Bam { foo: "hi hi", bar: None, baz: Err(2) };
    assert_uri_display_query!(bam, "foo=hi%20hi");

    let bam = Bam { foo: "hi hi", bar: Some(1), baz: Ok("tony".into()) };
    assert_uri_display_query!(bam, "foo=hi%20hi&bar=1&baz=tony");

    let bam = Bam { foo: "hi hi", bar: None, baz: Ok("tony".into()) };
    assert_uri_display_query!(bam, "foo=hi%20hi&baz=tony");
}

macro_rules! assert_uri_display_path {
    ($v:expr, $s:expr) => (
        let uri_string = format!("{}", &$v as &dyn UriDisplay<Path>);
        assert_eq!(uri_string, $s);
    )
}

#[derive(UriDisplayPath)]
struct FooP(&'static str);

#[derive(UriDisplayPath)]
struct BarP<'a>(&'a str);

#[derive(UriDisplayPath)]
struct BazP<'a, T>(&'a T);

#[derive(UriDisplayPath)]
struct BamP<T>(T);

#[derive(UriDisplayPath)]
struct BopP(FooP);

#[test]
fn uri_display_path() {
    assert_uri_display_path!(FooP("hi"), "hi");
    assert_uri_display_path!(FooP("hi there"), "hi%20there");
    assert_uri_display_path!(BarP("hi there"), "hi%20there");
    assert_uri_display_path!(BazP(&FooP("hi")), "hi");
    assert_uri_display_path!(BazP(&BarP("hi there")), "hi%20there");
    assert_uri_display_path!(BamP(12), "12");
    assert_uri_display_path!(BamP(BazP(&100)), "100");
    assert_uri_display_path!(BopP(FooP("bop foo")), "bop%20foo");
}
