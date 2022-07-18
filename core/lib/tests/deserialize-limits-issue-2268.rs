use rocket::config::Config;
use rocket::data::Limits;
use rocket::figment::{providers::Serialized, Figment};
use ubyte::ToByteUnit;

#[test]
fn deserialize_mixed_case_limits_should_work() {
    let figment = Figment::default()
        .merge(Serialized::default("key1", 1.kibibytes()))
        .merge(Serialized::default("key5", 5.kibibytes()))
        .merge(Serialized::default("key3", 3.kibibytes()))
        .merge(Serialized::default("Key2", 2.kibibytes()))
        .merge(Serialized::default("Key4", 4.kibibytes()))
        .merge(Serialized::default("Key6", 6.kibibytes()));

    let limits: Limits = figment.extract().unwrap();
    assert_eq!(limits.get("key1"), Some(1.kibibytes()));
    assert_eq!(limits.get("key2"), Some(2.kibibytes()));
    assert_eq!(limits.get("key3"), Some(3.kibibytes()));
    assert_eq!(limits.get("key4"), Some(4.kibibytes()));
    assert_eq!(limits.get("key5"), Some(5.kibibytes()));
    assert_eq!(limits.get("key6"), Some(6.kibibytes()));
}

#[test]
fn deserialize_extra_limits_in_config_should_work() {
    let extra_limits = Limits::new().limit("Phactory", 1.kibibytes());
    let figment = Config::figment().merge(("limits", extra_limits));
    let config = Config::from(figment);
    assert_eq!(config.limits.get("Phactory"), Some(1.kibibytes()));
}
