#![cfg(feature = "secrets")]
#![deny(warnings)]

#[cfg(test)]
mod cookies_private_tests {
    use rocket::config::{SecretKey, Cipher};

    #[test]
    fn cipher_conversions() {
        let secret_key = SecretKey::generate().unwrap();

        let plaintext = "I like turtles";
        let cipher = secret_key.encrypt(plaintext).unwrap();

        assert_eq!(cipher, Cipher::from_bytes(&cipher.as_bytes()));
        assert_eq!(cipher, Cipher::from_vec(cipher.clone().into_vec()));
        assert_eq!(cipher, Cipher::from_hex(&cipher.to_hex()).unwrap());
        assert_eq!(cipher, Cipher::from_base64(&cipher.to_base64()).unwrap());
    }

    #[test]
    fn encrypt_decrypt() {
        let secret_key = SecretKey::generate().unwrap();

        // encrypt byte array
        let msg = "very-secret-message".as_bytes();
        let encrypted = secret_key.encrypt(&msg).unwrap();
        let decrypted = secret_key.decrypt(&encrypted).unwrap();
        assert_eq!(msg, decrypted);

        // encrypt String
        let msg = "very-secret-message".to_string();
        let encrypted = secret_key.encrypt(&msg).unwrap();
        let decrypted = secret_key.decrypt(&encrypted).unwrap();
        assert_eq!(msg.as_bytes(), decrypted);
    }

    #[test]
    fn encrypt_with_wrong_key() {
        let msg = "very-secret-message".as_bytes();

        let secret_key = SecretKey::generate().unwrap();
        let encrypted = secret_key.encrypt(msg).unwrap();

        let another_secret_key = SecretKey::generate().unwrap();
        let result = another_secret_key.decrypt(&encrypted);
        assert!(result.is_err());
    }
}
