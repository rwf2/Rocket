#[macro_use] extern crate rocket;
use rocket::catcher::TypedError;
use rocket::http::Status;

fn boxed_error<'r>(_val: Box<dyn TypedError<'r> + 'r>) {}

#[derive(TypedError)]
pub enum Foo<'r> {
    First(String),
    Second(Vec<u8>),
    Third {
        #[error(source)]
        responder: std::io::Error,
    },
    #[error(status = 400)]
    Fourth {
        string: &'r str,
    },
}

#[test]
fn validate_foo() {
    let first = Foo::First("".into());
    assert_eq!(first.status(), Status::InternalServerError);
    assert!(first.source(0).is_none());
    boxed_error(Box::new(first));
    let second = Foo::Second(vec![]);
    assert_eq!(second.status(), Status::InternalServerError);
    assert!(second.source(0).is_none());
    boxed_error(Box::new(second));
    let third = Foo::Third {
        responder: std::io::Error::new(std::io::ErrorKind::NotFound, ""),
    };
    assert_eq!(third.status(), Status::InternalServerError);
    assert!(std::ptr::eq(
        third.source(0).unwrap(),
        if let Foo::Third { responder } = &third { responder } else { panic!() }
    ));
    boxed_error(Box::new(third));
    let fourth = Foo::Fourth { string: "" };
    assert_eq!(fourth.status(), Status::BadRequest);
    assert!(fourth.source(0).is_none());
    boxed_error(Box::new(fourth));
}

#[derive(TypedError)]
pub struct InfallibleError {
    #[error(source)]
    _inner: std::convert::Infallible,
}

#[derive(TypedError)]
pub struct StaticError {
    #[error(source)]
    inner: std::string::FromUtf8Error,
}

#[test]
fn validate_static() {
    let val = StaticError {
        inner: String::from_utf8(vec![0xFF]).unwrap_err(),
    };
    assert_eq!(val.status(), Status::InternalServerError);
    assert!(std::ptr::eq(
        val.source(0).unwrap(),
        &val.inner,
    ));
    boxed_error(Box::new(val));
}

#[derive(TypedError)]
pub enum Generic<E> {
    First(E),
}

#[derive(TypedError)]
pub struct GenericWithLifetime<'r, E> {
    s: &'r str,
    inner: E,
}
