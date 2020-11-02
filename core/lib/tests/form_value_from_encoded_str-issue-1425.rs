use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use rocket::request::FromFormValue;

macro_rules! assert_from_form_value_eq {
    ($string:literal as $T:ty, $expected:expr) => (
        let value: $T = FromFormValue::from_form_value($string.into()).unwrap();
        assert_eq!(value, $expected);
    )
}

#[test]
fn test_from_form_value_encoded() {
    assert_from_form_value_eq!(
        "127.0.0.1%3A80" as SocketAddrV4,
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 80)
    );

    assert_from_form_value_eq!(
        "2001%3A0db8%3A85a3%3A0000%3A0000%3A8a2e%3A0370%3A7334" as Ipv6Addr,
        Ipv6Addr::new(0x2001, 0x0db8, 0x85a3, 0, 0, 0x8a2e, 0x0370, 0x7334)
    );

    assert_from_form_value_eq!(
        "%5B2001%3Adb8%3A%3A1%5D%3A8080" as SocketAddrV6,
        SocketAddrV6::new(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 8080, 0, 0)
    );
}
