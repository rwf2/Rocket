use std::fmt::Debug;

use rocket::form::{Form, FromForm};
use rocket::form::error::{Error, Errors, ErrorKind};

#[derive(Debug, FromForm)]
#[allow(dead_code)]
struct Cat<'v> {
    #[field(validate = len(5..))]
    name: &'v str,
    #[field(validate = starts_with("kitty"))]
    nick: &'v str,
}

#[derive(Debug, FromForm)]
#[allow(dead_code)]
struct Dog<'v> {
    #[field(validate = len(5..))]
    name: &'v str,
}

#[derive(Debug, FromForm)]
#[allow(dead_code)]
struct Person<'v> {
    kitty: Cat<'v>,
    #[field(validate = len(1..))]
    cats: Vec<Cat<'v>>,
    dog: Dog<'v>,
}

fn starts_with<'v, S: AsRef<str>>(string: S, prefix: &str) -> Result<(), Errors<'v>> {
    if !string.as_ref().starts_with(prefix) {
        Err(Error::validation(format!("must start with {:?}", prefix)))?
    }

    Ok(())
}

#[track_caller]
fn errors<'v, T: FromForm<'v> + Debug + 'v>(string: &'v str) -> Errors<'v> {
    Form::<T>::parse(string).expect_err("expected an error")
}

#[test]
fn test_form_validation_context() {
    use ErrorKind::*;

    fn count<'a, K>(c: &Errors<'_>, n: &str, kind: K, fuzz: bool) -> usize
        where K: Into<Option<ErrorKind<'a>>>
    {
        let kind = kind.into();
        c.iter().filter(|e| {
            let matches = (fuzz && e.is_for(n)) || (!fuzz && e.is_for_exactly(n));
            let kinded = kind.as_ref().map(|k| k == &e.kind).unwrap_or(true);
            matches && kinded
        }).count()
    }

    fn fuzzy<'a, K>(c: &Errors<'_>, n: &str, kind: K) -> usize
        where K: Into<Option<ErrorKind<'a>>>
    {
        count(c, n, kind, true)
    }

    fn exact<'a, K>(c: &Errors<'_>, n: &str, kind: K) -> usize
        where K: Into<Option<ErrorKind<'a>>>
    {
        count(c, n, kind, false)
    }

    let c = errors::<Cat>("name=littlebobby");
    assert_eq!(exact(&c, "nick", Missing), 1);
    assert_eq!(fuzzy(&c, "nick", Missing), 1);
    assert_eq!(fuzzy(&c, "nick", None), 1);

    let c = errors::<Person>("cats[0].name=Bob");
    assert_eq!(exact(&c, "kitty", None), 1);
    assert_eq!(exact(&c, "kitty", Missing), 1);
    assert_eq!(exact(&c, "cats[0].nick", None), 1);
    assert_eq!(exact(&c, "cats[0].nick", Missing), 1);
    assert_eq!(exact(&c, "dog", None), 1);
    assert_eq!(exact(&c, "dog", Missing), 1);
    assert_eq!(exact(&c, "dog.name", None), 0);
    assert_eq!(exact(&c, "kitty.name", None), 0);
    assert_eq!(exact(&c, "kitty.nick", None), 0);

    assert_eq!(fuzzy(&c, "kitty", None), 1);
    assert_eq!(fuzzy(&c, "kitty.name", Missing), 1);
    assert_eq!(fuzzy(&c, "kitty.nick", Missing), 1);
    assert_eq!(fuzzy(&c, "cats[0].nick", Missing), 1);
    assert_eq!(fuzzy(&c, "dog.name", Missing), 1);
    assert_eq!(fuzzy(&c, "dog", None), 1);

    let c = errors::<Person>("cats[0].name=Bob&cats[0].nick=kit&kitty.name=Hi");
    assert_eq!(exact(&c, "kitty.nick", Missing), 1);
    assert_eq!(exact(&c, "kitty", None), 0);
    assert_eq!(exact(&c, "dog", Missing), 1);
    assert_eq!(exact(&c, "dog", None), 1);
    assert_eq!(exact(&c, "cats[0].name", None), 1);
    assert_eq!(exact(&c, "cats[0].name", InvalidLength { min: Some(5), max: None }), 1);
    assert_eq!(exact(&c, "cats[0].nick", None), 1);
    assert_eq!(exact(&c, "cats[0].nick", Validation("must start with \"kitty\"".into())), 1);

    assert_eq!(fuzzy(&c, "kitty.nick", Missing), 1);
    assert_eq!(fuzzy(&c, "kitty.nick", None), 1);
    assert_eq!(fuzzy(&c, "kitty", None), 0);
    assert_eq!(fuzzy(&c, "dog.name", Missing), 1);
    assert_eq!(fuzzy(&c, "dog", Missing), 1);
    assert_eq!(fuzzy(&c, "cats[0].nick", None), 1);
    assert_eq!(exact(&c, "cats[0].name", None), 1);

    let c = errors::<Person>("kitty.name=Michael");
    assert_eq!(exact(&c, "kitty.nick", Missing), 1);
    assert_eq!(exact(&c, "dog", Missing), 1);
    assert_eq!(exact(&c, "cats[0].name", None), 0);
    assert_eq!(exact(&c, "cats[0].nick", None), 0);

    assert_eq!(exact(&c, "cats", None), 1);
    assert_eq!(exact(&c, "cats", InvalidLength { min: Some(1), max: None }), 1);

    assert_eq!(fuzzy(&c, "kitty.nick", Missing), 1);
    assert_eq!(fuzzy(&c, "kitty.nick", None), 1);
    assert_eq!(fuzzy(&c, "dog", None), 1);
    assert_eq!(fuzzy(&c, "dog.name", Missing), 1);
    assert_eq!(exact(&c, "cats[0].name", None), 0);
    assert_eq!(exact(&c, "cats[0].nick", None), 0);

    let c = errors::<Person>("kitty.name=Michael&kitty.nick=kittykat&dog.name=woofy");
    assert_eq!(c.iter().count(), 1);
    assert_eq!(exact(&c, "cats", None), 1);
    assert_eq!(exact(&c, "cats", InvalidLength { min: Some(1), max: None }), 1);
    assert_eq!(fuzzy(&c, "cats[0].name", None), 1);
}

// #[derive(Debug, FromForm)]
// struct Person<'v> {
//     kitty: Cat<'v>,
//     #[field(validate = len(1..))]
//     cats: Vec<Cat<'v>>,
//     dog: Dog<'v>,
// }
