// Note this test should be the only test done separately from others
// as installing the crypto provider affect the whole process

use tls::{DEFAULT_PROFILES, validate_profiles};

#[test]
#[cfg(unix)]
fn validate_tls_profiles_for_aws_lc() {
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();
    validate_profiles(DEFAULT_PROFILES);
    validate_profiles(&["ecdsa_nistp521_sha512_pkcs8"]);
}
