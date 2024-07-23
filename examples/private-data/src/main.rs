#[macro_use]
extern crate rocket;

use rocket::config::Cipher;
use rocket::{Config, State};
use rocket::fairing::AdHoc;
use rocket::response::status;
use rocket::http::Status;

#[cfg(test)] mod tests;

#[get("/encrypt/<msg>")]
fn encrypt_endpoint(msg: &str, config: &State<Config>) -> Result<String, status::Custom<String>> {
    let secret_key = config.secret_key.clone();

    let encrypted_msg = secret_key
        .encrypt(msg)
        .map(|cipher| cipher.to_base64())
        .map_err(|_| {
            status::Custom(Status::InternalServerError, "Failed to encrypt message".to_string())
        })?;

    info!("received message for encrypt: '{}'", msg);
    info!("encrypted msg: '{}'", encrypted_msg);

    Ok(encrypted_msg)
}

#[get("/decrypt/<msg>")]
fn decrypt_endpoint(msg: &str, config: &State<Config>) -> Result<String, status::Custom<String>> {
    let secret_key = config.secret_key.clone();

    let cipher = Cipher::from_base64(msg).map_err(|_| {
        status::Custom(Status::BadRequest, "Failed to decode base64".to_string())
    })?;

    let decrypted = secret_key.decrypt(&cipher).map_err(|_| {
        status::Custom(Status::InternalServerError, "Failed to decrypt message".to_string())
    })?;

    let decrypted_msg = String::from_utf8(decrypted).map_err(|_| {
        status::Custom(Status::InternalServerError,
        "Failed to convert decrypted message to UTF-8".to_string())
    })?;

    info!("received message for decrypt: '{}'", msg);
    info!("decrypted msg: '{}'", decrypted_msg);

    Ok(decrypted_msg)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![encrypt_endpoint, decrypt_endpoint])
        .attach(AdHoc::config::<Config>())
}
