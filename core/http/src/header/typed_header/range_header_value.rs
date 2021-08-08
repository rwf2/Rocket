use std::convert::TryFrom;
use super::{
    http_rule_parser,
    RangeItemHeaderValue
};

/// Represents a *Range* header value.
///
/// The `RangeHeaderValue` provides support for the Range header as defined in
/// [RFC 2616](https://tools.ietf.org/html/rfc2616)
#[derive(Clone)]
pub struct RangeHeaderValue {
    /// The unit from the header.
    pub unit: Vec<char>,

    /// The ranges specified in the header.
    pub ranges: Vec<RangeItemHeaderValue>
}

impl RangeHeaderValue {

    /// Initializes a new struct of `RangeHeaderValue`.
    pub fn new(from: Option<u64>, to: Option<u64>) -> Self {
        let mut v = Self::default();
        v.ranges.push(RangeItemHeaderValue::new(from, to));
        v
    }
}

impl Default for RangeHeaderValue {
    fn default() -> Self {
        Self { unit: "bytes".chars().collect(), ranges: Vec::new() }
    }
}


impl TryFrom<Vec<&str>> for RangeHeaderValue {
    type Error = ();

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        let start_index = 0;

        // Parse the unit string: <unit> in '<unit>=<from1>-<to1>, <from2>-<to2>'
        let input = value[0].chars().collect::<Vec<char>>();
        let unit_len = http_rule_parser::get_token_length(&input, start_index);
        if unit_len == 0 {
            return Err(());
        }
        let mut result = Self{
            unit: input[start_index..start_index + unit_len].to_owned(),
            ..Self::default()
        };
        let mut current = start_index + unit_len;
        current += http_rule_parser::get_whitespace_length(&input, current);
        if current == input.len() || input[current] != '=' {
            return Err(())
        }
        current += 1; // skip '=' separator
        current += http_rule_parser::get_whitespace_length(&input, current);
        let ranges_len = RangeItemHeaderValue::get_range_item_list_length(&input, current, &mut result.ranges);

        if ranges_len == 0 {
            return Err(());
        }
        current += ranges_len;

        assert_eq!(current, input.len());

        Ok(result)
    }
}


impl ToString for RangeHeaderValue {
    fn to_string(&self) -> String {
        let mut s = String::new();
        self.unit.iter().for_each(|u|s.push(*u));
        s.push('=');

        for (i, range) in self.ranges.iter().enumerate() {
            if i > 0 {
                s.push(',');
                s.push(' ');
            }
            if let Some(ref f) = range.from {
                s.push_str(f.to_string().as_str());
            }
            s.push('-');
            if let Some(ref t) = range.to {
                s.push_str(t.to_string().as_str());
            }
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::super::HeaderMap;

    #[test]
    fn test_get_range_header_value_u_s() {
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::RANGE, "bytes=0-");
        let value = headers.get_typed_headers().range().unwrap();

        assert_eq!(0, value.ranges[0].from.unwrap());
        assert_eq!(None, value.ranges[0].to);
        assert_eq!("bytes=0-", value.to_string());
    }

    #[test]
    fn test_get_range_header_value_u_s1() {
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::RANGE, "bytes = 0 -");
        let value = headers.get_typed_headers().range().unwrap();

        assert_eq!(0, value.ranges[0].from.unwrap());
        assert_eq!(None, value.ranges[0].to);
        assert_eq!("bytes=0-", value.to_string());
    }

    #[test]
    fn test_get_range_header_value_u_s_e() {
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::RANGE, "bytes=7-9");
        let value = headers.get_typed_headers().range().unwrap();

        assert_eq!(7, value.ranges[0].from.unwrap());
        assert_eq!(9, value.ranges[0].to.unwrap());
        assert_eq!("bytes=7-9", value.to_string());
    }

    #[test]
    fn test_get_range_header_value_u_s_e_2() {
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::RANGE, "bytes=7-9,11-13");
        let value = headers.get_typed_headers().range().unwrap();

        assert_eq!(7, value.ranges[0].from.unwrap());
        assert_eq!(9, value.ranges[0].to.unwrap());
        assert_eq!(11, value.ranges[1].from.unwrap());
        assert_eq!(13, value.ranges[1].to.unwrap());
        assert_eq!("bytes=7-9, 11-13", value.to_string());
    }
}