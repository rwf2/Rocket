//! NaiveDateTime (ISO 8601) parameter and form value parsing support.
//!
//! See the [`chrono::NaiveDateTime`] type for further details.
//!
//! # Enabling
//!
//! This module is only available when the `NaiveDateTime` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["datetime"]
//! ```

pub extern crate chrono as chrono_crate;

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use rocket::http::RawStr;
use rocket::request::{FromFormValue, FromParam};

type ParseError = self::chrono_crate::format::ParseError;

/// Implements [`FromParam`] and [`FromFormValue`] for accepting ISO-8601 formatted DateTime values without timezones.
///
/// # Usage
///
/// To use, add the `NaiveDateTime` feature to the `rocket_contrib` dependencies section
/// of your `Cargo.toml`:
///
/// ```toml
/// [dependencies.rocket_contrib]
/// version = "0.5.0-dev"
/// default-features = false
/// features = ["datetime"]
/// ```
///
/// You can use the `NaiveDateTime` type directly as a target of a dynamic parameter:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[macro_use] extern crate rocket_contrib;
/// use rocket_contrib::datetime::NaiveDateTime;
///
/// #[get("/logs/<datetime>")]
/// fn get_logs(datetime: NaiveDateTime) -> String {
///     format!("We found: {}", datetime)
/// }
/// ```
///
/// You can also use the `NaiveDateTime` as a form value, including in query strings:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[macro_use] extern crate rocket_contrib;
/// use rocket_contrib::datetime::NaiveDateTime;
///
/// #[get("/logs?<datetime>")]
/// fn logs(datetime: NaiveDateTime) -> String {
///     format!("What time is it Mr. Wolf? It's {}", datetime)
/// }
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct NaiveDateTime(chrono_crate::NaiveDateTime);

impl NaiveDateTime {
    /// Consumes the NaiveDateTime wrapper, returning the underlying `chrono::NaiveDateTime` type.
    ///
    /// # Example
    /// ```rust
    /// # use std::str::FromStr;
    /// # fn main() {
    /// use rocket_contrib::datetime::{chrono_crate, NaiveDateTime};
    ///
    /// let iso8601_str = "2020-01-01T10:30:45";
    /// let real_datetime = chrono_crate::NaiveDateTime::from_str(iso8601_str).unwrap();
    /// let my_inner_datetime = NaiveDateTime::from_str(iso8601_str)
    ///     .expect("valid NaiveDateTime string")
    ///     .into_inner();
    ///
    /// assert_eq!(real_datetime, my_inner_datetime);
    /// # }
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> chrono_crate::NaiveDateTime {
        self.0
    }
}

impl fmt::Display for NaiveDateTime {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> FromParam<'a> for NaiveDateTime {
    type Error = ParseError;

    /// A value is successfully parsed if `param` is a properly formatted NaiveDateTime.
    /// Otherwise, a `ParseError` is returned.
    #[inline(always)]
    fn from_param(param: &'a RawStr) -> Result<NaiveDateTime, Self::Error> {
        param.parse()
    }
}

impl<'v> FromFormValue<'v> for NaiveDateTime {
    type Error = &'v RawStr;

    /// A value is successfully parsed if `form_value` is a properly formatted
    /// NaiveDateTime. Otherwise, the raw form value is returned.
    #[inline(always)]
    fn from_form_value(form_value: &'v RawStr) -> Result<NaiveDateTime, &'v RawStr> {
        form_value.parse().map_err(|_| form_value)
    }
}

impl FromStr for NaiveDateTime {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<NaiveDateTime, Self::Err> {
        s.parse().map(NaiveDateTime)
    }
}

impl Deref for NaiveDateTime {
    type Target = chrono_crate::NaiveDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<chrono_crate::NaiveDateTime> for NaiveDateTime {
    #[inline(always)]
    fn eq(&self, other: &chrono_crate::NaiveDateTime) -> bool {
        self.0.eq(other)
    }
}

#[cfg(test)]
mod test {
    use super::chrono_crate;
    use super::FromParam;
    use super::FromStr;
    use super::NaiveDateTime;

    #[test]
    fn test_from_str() {
        let datetime_str = "2011-12-03T10:15:30";
        let datetime_wrapper = NaiveDateTime::from_str(datetime_str).unwrap();
        assert_eq!("2011-12-03 10:15:30", datetime_wrapper.to_string())
    }

    #[test]
    fn test_from_param() {
        let datetime_str = "2011-12-03T10:15:30";
        let datetime_wrapper = NaiveDateTime::from_param(datetime_str.into()).unwrap();
        assert_eq!("2011-12-03 10:15:30", datetime_wrapper.to_string())
    }

    #[test]
    fn test_into_inner() {
        let datetime_str = "2011-12-03T10:15:30";
        let datetime_wrapper = NaiveDateTime::from_param(datetime_str.into()).unwrap();
        let real_datetime: chrono_crate::NaiveDateTime = datetime_str.parse().unwrap();
        let inner_datetime: chrono_crate::NaiveDateTime = datetime_wrapper.into_inner();
        assert_eq!(real_datetime, inner_datetime)
    }

    #[test]
    fn test_partial_eq() {
        let datetime_str = "2011-12-03T10:15:30";
        let datetime_wrapper = NaiveDateTime::from_param(datetime_str.into()).unwrap();
        let real_datetime: chrono_crate::NaiveDateTime = datetime_str.parse().unwrap();
        assert_eq!(datetime_wrapper, real_datetime)
    }

    #[test]
    fn test_inner_eq() {
        let iso8601_str = "2020-01-01T10:30:45";
        let real_datetime = chrono_crate::NaiveDateTime::from_str(iso8601_str).unwrap();

        let my_inner_datetime = NaiveDateTime::from_str(iso8601_str)
            .expect("valid NaiveDateTime string")
            .into_inner();
        assert_eq!(real_datetime, my_inner_datetime);
    }

    #[test]
    #[should_panic]
    fn test_from_param_invalid() {
        let datetime_str = "2011-12-03";
        NaiveDateTime::from_param(datetime_str.into()).unwrap();
    }
}
