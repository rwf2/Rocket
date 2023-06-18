#![cfg(feature = "msgpack")]

use rocket::{Rocket, Build};
use rocket::serde::msgpack;
use rocket::local::blocking::Client;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Person {
    name: String,
    age: u8,
    gender: Gender,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "gender")]
enum Gender {
    Male,
    Female,
    NonBinary,
}

#[rocket::post("/age_named", data = "<person>")]
fn named(person: msgpack::MsgPack<Person>) -> msgpack::Named<Person> {
    let person = Person { age: person.age + 1, ..person.into_inner() };
    msgpack::MsgPack(person)
}

#[rocket::post("/age_compact", data = "<person>")]
fn compact(person: msgpack::MsgPack<Person>) -> msgpack::Compact<Person> {
    let person = Person { age: person.age + 1, ..person.into_inner() };
    msgpack::MsgPack(person)
}

fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", rocket::routes![named, compact])
}

fn read_string(buf: &mut rmp::decode::Bytes) -> String {
    let mut string_buf = vec![0; 32];  // Awful but we're just testing.
    rmp::decode::read_str(buf, &mut string_buf).unwrap().to_string()
}

#[test]
fn check_named_roundtrip() {
    let client = Client::debug(rocket()).unwrap();
    let person = Person {
        name: "Cal".to_string(),
        age: 17,
        gender: Gender::NonBinary,
    };
    let response = client
        .post("/age_named")
        .body(rmp_serde::to_vec_named(&person).unwrap())
        .dispatch()
        .into_bytes()
        .unwrap();
    let mut bytes = rmp::decode::Bytes::new(&response);
    assert_eq!(rmp::decode::read_map_len(&mut bytes).unwrap(), 3);
    assert_eq!(&read_string(&mut bytes), "name");
    assert_eq!(&read_string(&mut bytes), "Cal");
    assert_eq!(&read_string(&mut bytes), "age");
    assert_eq!(rmp::decode::read_int::<u8, _>(&mut bytes).unwrap(), 18);
    assert_eq!(&read_string(&mut bytes), "gender");
    // Enums are complicated in serde. In this test, they're encoded like this:
    // (JSON equivalent) `{ "gender": "NonBinary" }`, where that object is itself
    // the value of the `gender` key in the outer object. `#[serde(flatten)]`
    // on the `gender` key in the outer object fixes this, but it prevents `rmp`
    // from using compact mode, which would break the test.
    assert_eq!(rmp::decode::read_map_len(&mut bytes).unwrap(), 1);
    assert_eq!(&read_string(&mut bytes), "gender");
    assert_eq!(&read_string(&mut bytes), "NonBinary");
}

#[test]
fn check_compact_roundtrip() {
    let client = Client::debug(rocket()).unwrap();
    let person = Person {
        name: "Maeve".to_string(),
        age: 15,
        gender: Gender::Female,
    };
    let response = client
        .post("/age_compact")
        .body(rmp_serde::to_vec(&person).unwrap())
        .dispatch()
        .into_bytes()
        .unwrap();
    let mut bytes = rmp::decode::Bytes::new(&response);
    assert_eq!(rmp::decode::read_array_len(&mut bytes).unwrap(), 3);
    assert_eq!(&read_string(&mut bytes), "Maeve");
    assert_eq!(rmp::decode::read_int::<u8, _>(&mut bytes).unwrap(), 16);
    // Equivalent to the named representation, gender here is encoded like this:
    // `[ "Female" ]`.
    assert_eq!(rmp::decode::read_array_len(&mut bytes).unwrap(), 1);
    assert_eq!(&read_string(&mut bytes), "Female");
}
