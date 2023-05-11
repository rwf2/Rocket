use rocket::local::blocking::Client;
use rocket::http::{RawStr, Status};

#[test]
fn hello() {
    let langs = &["", "ua", "%D1%83%D0%B0", "en", "unknown"];
    let ex_lang = &["Hi", "Привіт", "Привіт", "Hello", "Hi"];

    let emojis = &["", "on", "true", "false", "no", "yes", "off"];
    let ex_emoji = &["", "👋 ", "👋 ", "", "", "👋 ", ""];

    let names = &["", "Bob", "Bob+Smith"];
    let ex_name = &["!", ", Bob!", ", Bob Smith!"];

    let client = Client::tracked(super::rocket()).unwrap();
    for n in 0..(langs.len() * emojis.len() * names.len()) {
        let i = n / (emojis.len() * names.len());
        let j = n % (emojis.len() * names.len()) / names.len();
        let k = n % (emojis.len() * names.len()) % names.len();

        let (lang, ex_lang) = (langs[i], ex_lang[i]);
        let (emoji, ex_emoji) = (emojis[j], ex_emoji[j]);
        let (name, ex_name) = (names[k], ex_name[k]);
        let expected = format!("{}{}{}", ex_emoji, ex_lang, ex_name);

        let q = |name, s: &str| match s.is_empty() {
            true => "".into(),
            false => format!("&{}={}", name, s)
        };

        let uri = format!("/?{}{}{}", q("lang", lang), q("emoji", emoji), q("name", name));
        let response = client.get(uri).dispatch();
        assert_eq!(response.into_string().unwrap(), expected);

        let uri = format!("/?{}{}{}", q("emoji", emoji), q("name", name), q("lang", lang));
        let response = client.get(uri).dispatch();
        assert_eq!(response.into_string().unwrap(), expected);
    }
}

#[test]
fn hello_world() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/hello/world").dispatch();
    assert_eq!(response.into_string(), Some("Hello, world!".into()));
}

#[test]
fn hello_mir() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/hello/%D1%81%D0%B2%D1%96%D1%82").dispatch();
    assert_eq!(response.into_string(), Some("Привіт, світе!".into()));
}

#[test]
fn wave() {
    let client = Client::tracked(super::rocket()).unwrap();
    for &(name, age) in &[("Bob%20Smith", 22), ("Michael", 80), ("A", 0), ("a", 127)] {
        let uri = format!("/wave/{}/{}", name, age);
        let real_name = RawStr::new(name).percent_decode_lossy();
        let expected = format!("👋 Hello, {} year old named {}!", age, real_name);
        let response = client.get(uri).dispatch();
        assert_eq!(response.into_string().unwrap(), expected);

        for bad_age in &["1000", "-1", "bird", "?"] {
            let bad_uri = format!("/wave/{}/{}", name, bad_age);
            let response = client.get(bad_uri).dispatch();
            assert_eq!(response.status(), Status::NotFound);
        }
    }
}
