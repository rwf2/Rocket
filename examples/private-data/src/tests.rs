use rocket::{config::SecretKey, local::blocking::Client};

#[test]
fn encrypt_decrypt() {
    let secret_key = SecretKey::generate().unwrap();
    let msg = "very-secret-message".as_bytes();

    let encrypted = secret_key.encrypt(msg).unwrap();
    let decrypted = secret_key.decrypt(&encrypted).unwrap();

    assert_eq!(msg, decrypted);
}

#[test]
fn encrypt_decrypt_api() {
    let client = Client::tracked(super::rocket()).unwrap();
    let msg = "some-secret-message";

    let encrypted = client.get(format!("/encrypt/{}", msg)).dispatch().into_string().unwrap();
    let decrypted = client.get(format!("/decrypt/{}", encrypted)).dispatch().into_string().unwrap();

    assert_eq!(msg, decrypted);
}
