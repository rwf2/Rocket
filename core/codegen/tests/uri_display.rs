#[macro_use] extern crate rocket;

use rocket::http::uri::fmt::{UriDisplay, Query, Path};
use rocket::serde::{Serialize, Deserialize};

macro_rules! assert_uri_display_query {
    ($v:expr, $expected:expr) => (
        let uri_string = format!("{}", &$v as &dyn UriDisplay<Query>);
        assert_eq!(uri_string, $expected);
    )
}

macro_rules! assert_query_form_roundtrip {
    ($T:ty, $v:expr) => ({
        use rocket::form::{Form, Strict};
        use rocket::http::RawStr;

        let v = $v;
        let string = format!("{}", &v as &dyn UriDisplay<Query>);
        let raw = RawStr::new(&string);
        let value = Form::<Strict<$T>>::parse_encoded(raw).map(|s| s.into_inner());
        assert_eq!(value.expect("form parse"), v);
    })
}

macro_rules! assert_query_value_roundtrip {
    ($T:ty, $v:expr) => ({
        use rocket::form::{Form, Strict};
        use rocket::http::RawStr;

        let v = $v;
        let string = format!("={}", &v as &dyn UriDisplay<Query>);
        let raw = RawStr::new(&string);
        let value = Form::<Strict<$T>>::parse_encoded(raw).map(|s| s.into_inner());
        assert_eq!(value.expect("form parse"), v);
    })
}

#[derive(UriDisplayQuery, Clone)]
enum Foo<'r> {
    First(&'r str),
    Second {
        inner: &'r str,
        other: usize,
    },
    Third {
        #[field(name = "type")]
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
    baz: Result<&'a str, usize>,
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

#[test]
fn uri_display_c_like() {
    #[derive(UriDisplayQuery)]
    enum CLike { A, B, C }

    assert_uri_display_query!(CLike::A, "A");
    assert_uri_display_query!(CLike::B, "B");
    assert_uri_display_query!(CLike::C, "C");

    #[derive(UriDisplayQuery)]
    enum CLikeV {
        #[field(value = "a")]
        A,
        #[field(value = "tomato")]
        #[field(value = "juice")]
        B,
        #[field(value = "carrot")]
        C
    }

    assert_uri_display_query!(CLikeV::A, "a");
    assert_uri_display_query!(CLikeV::B, "tomato");
    assert_uri_display_query!(CLikeV::C, "carrot");

    #[derive(UriDisplayQuery)]
    #[allow(non_camel_case_types)]
    enum CLikeR { r#for, r#type, r#async, #[field(value = "stop")] r#yield }

    assert_uri_display_query!(CLikeR::r#for, "for");
    assert_uri_display_query!(CLikeR::r#type, "type");
    assert_uri_display_query!(CLikeR::r#async, "async");
    assert_uri_display_query!(CLikeR::r#yield, "stop");

    #[derive(UriDisplayQuery)]
    struct Nested {
        foo: CLike,
        bar: CLikeV,
        last: CLikeR
    }

    let nested = Nested {
        foo: CLike::B,
        bar: CLikeV::B,
        last: CLikeR::r#type,
    };

    assert_uri_display_query!(nested, "foo=B&bar=tomato&last=type");
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

#[test]
fn uri_display_serde() {
    use rocket::serde::json::Json;

    #[derive(Debug, PartialEq, Clone, FromForm, UriDisplayQuery, Deserialize, Serialize)]
    #[serde(crate = "rocket::serde")]
    struct Bam {
        foo: String,
        bar: Option<usize>,
        baz: bool,
    }

    #[derive(Debug, PartialEq, FromForm, UriDisplayQuery)]
    struct JsonFoo(Json<Bam>);

    let bam = Bam {
        foo: "hi[]=there.baz !?".into(),
        bar: None,
        baz: true,
    };

    assert_query_form_roundtrip!(Bam, bam.clone());

    assert_query_value_roundtrip!(JsonFoo, JsonFoo(Json(bam.clone())));

    #[allow(renamed_and_removed_lints)]
    #[allow(private_in_public)]
    #[derive(Debug, PartialEq, Clone, FromForm, UriDisplayQuery)]
    struct Q<T>(Json<T>);

    #[derive(Debug, PartialEq, Clone, FromForm, UriDisplayQuery)]
    pub struct Generic<A, B> {
        a: Q<A>,
        b: Q<B>,
        c: Q<A>,
    }

    assert_query_form_roundtrip!(Generic<usize, String>, Generic {
        a: Q(Json(133)),
        b: Q(Json("hello, world#rocket!".into())),
        c: Q(Json(40486)),
    });

    #[derive(Debug, PartialEq, Clone, FromForm, UriDisplayQuery)]
    // This is here to ensure we don't warn, which we can't test with trybuild.
    pub struct GenericBorrow<'a, A: ?Sized, B: 'a> {
        a: Q<&'a A>,
        b: Q<B>,
        c: Q<&'a A>,
    }

    // TODO: This requires `MsgPack` to parse from value form fields.
    //
    // use rocket::serde::msgpack::MsgPack;
    //
    // #[derive(Debug, PartialEq, FromForm, UriDisplayQuery)]
    // struct MsgPackFoo(MsgPack<Bam>);
    //
    // assert_query_value_roundtrip!(MsgPackFoo, MsgPackFoo(MsgPack(bam)));
}
