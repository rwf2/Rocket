extern crate rocket;

use rocket::http::RawStr;
use rocket::request::FromFormValue;
use std::fmt::Debug;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

mod tests {
    use super::*;

    fn check_from_form_value_encoded<'a, T>(encoded_str: &'static str, expected: T)
    where
        T: FromFormValue<'a> + PartialEq + Debug,
        <T as FromFormValue<'a>>::Error: Debug,
    {
        let value = T::from_form_value(RawStr::from_str(encoded_str));

        assert!(value.is_ok());
        assert_eq!(value.unwrap(), expected);
    }

    #[test]
    fn test_from_form_value_encoded() {
        check_from_form_value_encoded(
            "127.0.0.1%3A80",
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 80),
        );

        check_from_form_value_encoded(
            "2001%3A0db8%3A85a3%3A0000%3A0000%3A8a2e%3A0370%3A7334",
            Ipv6Addr::new(0x2001, 0x0db8, 0x85a3, 0, 0, 0x8a2e, 0x0370, 0x7334),
        );

        check_from_form_value_encoded(
            "%5B2001%3Adb8%3A%3A1%5D%3A8080",
            SocketAddrV6::new(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 8080, 0, 0),
        );
    }
}
