extern crate uuid as uuid_ext;

use std::str::FromStr;
use std::ops::{Deref, DerefMut};
use rocket::request::FromParam;

pub struct UUID(uuid_ext::Uuid);

impl UUID {
    pub fn unwrap(self) -> uuid_ext::Uuid {
        self.0
    }
}

impl<'a> FromParam<'a> for UUID {
    type Error = uuid_ext::ParseError;
    fn from_param(p: &'a str) -> Result<UUID, Self::Error> {
        p.parse()
    }
}

impl FromStr for UUID {
    type Err = uuid_ext::ParseError;
    fn from_str(s: &str) -> Result<UUID, Self::Err> {
        Ok(UUID(try!(s.parse())))
    }
}

impl Deref for UUID {
    type Target = uuid_ext::Uuid;

    fn deref<'a>(&'a self) -> &'a Self::Target {
        &self.0
    }
}

impl DerefMut for UUID {
    fn deref_mut<'a>(&'a mut self) -> &'a mut uuid_ext::Uuid {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    use super::uuid_ext;
    use super::FromParam;
    use super::UUID;

    #[test]
    fn test_from_str() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid_result = uuid_str.parse();
        assert!(uuid_result.is_ok());
        let uuid:UUID = uuid_result.unwrap();
        assert!(uuid_str.to_string() == uuid.to_string())
    }

    #[test]
    fn test_from_param() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid_result = UUID::from_param(uuid_str);
        assert!(uuid_result.is_ok());
        let uuid = uuid_result.unwrap();
        assert!(uuid_str.to_string() == uuid.to_string())
    }

    #[test]
    fn test_from_param_invalid() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2p";
        let uuid_result = UUID::from_param(uuid_str);
        assert!(!uuid_result.is_ok());
        match uuid_result {
            Err(e) => assert_eq!(e, uuid_ext::ParseError::InvalidLength(37)),
            _ => unreachable!(),
        }
    }
}
