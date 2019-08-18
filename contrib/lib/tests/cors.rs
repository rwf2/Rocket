
// TODO Test with an allow-any origin
// TODO Test with two origins
// TODO Test with one origin
// TODO Test a variety of methods (delete, put, post)
// TODO Test with headers
// TODO Test with no headers
// TODO Test with multiple headers
// note, I think all of these can be integration tests
#[cfg(feature = "cors")]
mod cors_tests {
    use rocket_contrib::cors::*;

    #[test]
    pub fn test_basic() {
        let _ = CorsFairingBuilder::new();
    }
}
