#![cfg(feature = "msgpack")]

use std::borrow::Cow;

use rocket::{Rocket, Build};
use rocket::serde::msgpack::{self, MsgPack, Compact};
use rocket::local::blocking::Client;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Person<'r> {
    name: &'r str,
    age: u8,
    gender: Gender,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
enum Gender {
    Male,
    Female,
    NonBinary,
}

#[rocket::post("/named", data = "<person>")]
fn named(person: MsgPack<Person<'_>>) -> MsgPack<Person<'_>> {
    person
}

#[rocket::post("/compact", data = "<person>")]
fn compact(person: MsgPack<Person<'_>>) -> Compact<Person<'_>> {
    MsgPack(person.into_inner())
}

fn rocket() -> Rocket<Build> {
    rocket::build().mount("/", rocket::routes![named, compact])
}

// The object we're going to roundtrip through the API.
const OBJECT: Person<'static> = Person {
    name: "Cal",
    age: 17,
    gender: Gender::NonBinary,
};

// [ "Cal", 17, "NonBinary" ]
const COMPACT_BYTES: &[u8] = &[
    147, 163, 67, 97, 108, 17, 169, 78, 111, 110, 66, 105, 110, 97, 114, 121
];

// { "name": "Cal", "age": 17, "gender": "NonBinary" }
const NAMED_BYTES: &[u8] = &[
    131, 164, 110, 97, 109, 101, 163, 67, 97, 108, 163, 97, 103, 101, 17, 166,
    103, 101, 110, 100, 101, 114, 169, 78, 111, 110, 66, 105, 110, 97, 114, 121
];

#[test]
fn check_roundtrip() {
    let client = Client::debug(rocket()).unwrap();
    let inputs: &[(&'static str, Cow<'static, [u8]>)] = &[
        ("objpack", msgpack::to_vec(&OBJECT).unwrap().into()),
        ("named bytes", NAMED_BYTES.into()),
        ("compact bytes", COMPACT_BYTES.into()),
    ];

    for (name, input) in inputs {
        let compact = client.post("/compact").body(input).dispatch();
        assert_eq!(compact.into_bytes().unwrap(), COMPACT_BYTES, "{name} mismatch");

        let named = client.post("/named").body(input).dispatch();
        assert_eq!(named.into_bytes().unwrap(), NAMED_BYTES, "{name} mismatch");
    }
}
