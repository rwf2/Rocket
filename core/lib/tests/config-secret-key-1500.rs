use rocket::figment::Figment;
use rocket::config::{Config, SecretKey};

#[test]
fn secret_key_in_config_not_zero() {
    let original_key = SecretKey::generate().expect("get key");

    let config = Config { secret_key: original_key.clone(), ..Default::default() };
    let figment = Figment::from(config);
    let figment_key: SecretKey = figment.extract_inner("secret_key").unwrap();
    assert_eq!(original_key, figment_key);
}
