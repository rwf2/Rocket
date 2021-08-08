use std::convert::TryFrom;

use chrono::{DateTime, FixedOffset, Utc};
use std::time::{SystemTime, Duration};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};

/// Represents `DateTimeOffset` in HTTP header.
#[derive(Eq)]
pub struct DateTimeOffset(DateTime<FixedOffset>);

impl DateTimeOffset {

    /// Returns the DateTimeOffset corresponding to "now".
    pub fn now() -> Self {
        SystemTime::now().into()
    }

    /// Returns the number of non-leap-milliseconds since January 1, 1970 UTC
    pub fn timestamp_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }
}

impl Debug for DateTimeOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl ToString for DateTimeOffset {
    fn to_string(&self) -> String {
        let s: String = self.0.to_rfc2822();
        let mut s = s.trim_end_matches("+0000").to_string();
        s.push_str("GMT");
        s
    }
}

impl TryFrom<Vec<&str>> for DateTimeOffset {
    type Error = ();

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(())
        }

        let value: Vec<char> = value[0].chars().collect();

        Self::try_from(&value[..]).map_err(|_| ())
    }
}

impl TryFrom<&[char]> for DateTimeOffset {
    type Error = chrono::ParseError;

    fn try_from(chars: &[char]) -> Result<Self, Self::Error> {
        let s: String = chars.iter().collect();
        DateTime::parse_from_rfc2822(s.as_str())
            .map(Self::from)
    }
}

impl From<DateTime<FixedOffset>> for DateTimeOffset {
    fn from(time: DateTime<FixedOffset>) -> Self {
        Self(time)
    }
}

impl From<DateTime<Utc>> for DateTimeOffset {
    fn from(time: DateTime<Utc>) -> Self {
        Self(time.into())
    }
}

impl From<SystemTime> for DateTimeOffset {
    fn from(time: SystemTime) -> Self {
        let u: DateTime<Utc> = DateTime::from(time);
        u.into()
    }
}

impl From<u64> for DateTimeOffset {
    fn from(v: u64) -> Self {
        (SystemTime::UNIX_EPOCH + Duration::from_millis(v)).into()
    }
}

impl std::cmp::PartialEq for DateTimeOffset {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl std::cmp::PartialOrd for DateTimeOffset {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        let p = DateTimeOffset::from(1628392450871);
        assert_eq!("Sun, 08 Aug 2021 03:14:10 GMT", p.to_string());
    }
}