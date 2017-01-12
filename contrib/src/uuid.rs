extern crate uuid as uuid_ext;

use std::fmt;
use std::str::FromStr;
use std::ops::{Deref, DerefMut};
use rocket::request::FromParam;

/// The UUID type, which implements `FromParam`. This type allows you to accept
/// values from the [Uuid](https://github.com/rust-lang-nursery/uuid) crate as
/// a dynamic parameter in your request handlers.
///
/// # Usage
///
/// To use, add the `uuid` feature to the `rocket_contrib` dependencies section
/// of your `Cargo.toml`:
///
/// ```toml,ignore
/// [dependencies.rocket_contrib]
/// version = "*"
/// default-features = false
/// features = ["uuid"]
/// ```
///
/// The UUID type implements the Deref trait allowing you to access the
/// underlying Uuid type.
///
/// ```rust,ignore
/// #[get("/users/<id>")]
/// fn user(id: UUID) -> String {
///     // Use Deref to access the underlying Uuid type.
///     expects_uuid(*id);
///
///     format!("We found: {}", id)
/// }
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct UUID(uuid_ext::Uuid);

impl UUID {
    /// Consumes the UUID wrapper returning the underlying Uuid type.
    ///
    /// # Example
    /// ```rust,ignore
    /// # extern crate uuid;
    /// # use rocket_contrib::UUID;
    /// # use std::str::FromStr;
    ///
    /// let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
    /// let my_inner_uuid = UUID::from_str(uuid_str).unwrap().into_inner();
    /// assert_eq!(uuid::Uuid::from_str(uuid_str), my_inner_uuid)
    /// ```
    pub fn into_inner(self) -> uuid_ext::Uuid {
        self.0
    }
}

impl fmt::Display for UUID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
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
        use std::str::FromStr;
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid_wrapper = UUID::from_str(uuid_str).unwrap();
        assert_eq!(uuid_str, uuid_wrapper.to_string())
    }

    #[test]
    fn test_from_param() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid_wrapper = UUID::from_param(uuid_str).unwrap();
        assert_eq!(uuid_str, uuid_wrapper.to_string())
    }

    #[test]
    fn test_into_inner() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid_wrapper = UUID::from_param(uuid_str).unwrap();
        let real_uuid: uuid_ext::Uuid = uuid_str.parse().unwrap();
        let inner_uuid: uuid_ext::Uuid = uuid_wrapper.into_inner();
        assert_eq!(real_uuid, inner_uuid)
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
