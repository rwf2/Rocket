use std::convert::TryFrom;
use super::{
    EntityTagHeaderValue, DateTimeOffset
};
use std::result::Result::Ok;

/// Represents an `If-Range` header value which can either be a date/time or an entity-tag value.
#[derive(Clone)]
pub enum RangeConditionHeaderValue {

    /// A date value used to initialize the new instance.
    LastModified(DateTimeOffset),

    /// An entity tag uniquely representing the requested resource.
    EntityTag(EntityTagHeaderValue)
}

impl RangeConditionHeaderValue {

    /// Gets the LastModified date from header.
    pub fn last_modified(&self) -> Option<&DateTimeOffset> {
        match self {
            RangeConditionHeaderValue::LastModified(date) => Some(date),
            RangeConditionHeaderValue::EntityTag(_) => None
        }
    }

    /// Gets the `EntityTagHeaderValue` from header.
    pub fn entity_tag(&self) -> Option<&EntityTagHeaderValue> {
        match self {
            RangeConditionHeaderValue::LastModified(_) => None,
            RangeConditionHeaderValue::EntityTag(etag) => Some(etag)
        }
    }
}

impl TryFrom<Vec<&str>> for RangeConditionHeaderValue {
    type Error = ();

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        let start_index = 0;

        // Parse the unit string: <unit> in '<unit>=<from1>-<to1>, <from2>-<to2>'
        let input = value[0].chars().collect::<Vec<char>>();

        if input.is_empty() || start_index + 1 >= input.len() {
            return Err(());
        }

        let mut current = start_index;

        // Caller must remove leading whitespaces.
        let mut entity_tag = EntityTagHeaderValue::any();

        let first_char = input[current];
        let second_char = input[current + 1];
        if (first_char == '"') || (((first_char == 'w') || (first_char == 'W')) && (second_char == '/')) {
            // trailing whitespaces are removed by GetEntityTagLength()
            let entity_tag_length = EntityTagHeaderValue::get_entity_tag_length(
                &input, current,
                &mut entity_tag);

            if entity_tag_length == 0 {
                return Err(());
            }

            current += entity_tag_length;

            // RangeConditionHeaderValue only allows 1 value. There must be no delimiter/other chars after an
            // entity tag.
            if current != input.len() {

                return Err(());
            }
            Ok(Self::EntityTag(entity_tag) )
        } else if let Ok(date) = DateTimeOffset::try_from(&input[current..]) {
            // If we got a valid date, then the parser consumed the whole string (incl. trailing whitespaces).
            // current = input.len();
            Ok(Self::LastModified(date))
        } else {
            Err(())
        }
    }
}

impl ToString for RangeConditionHeaderValue {
    fn to_string(&self) -> String {
        match self {
            RangeConditionHeaderValue::LastModified(date) => date.to_string(),
            RangeConditionHeaderValue::EntityTag(etag) => etag.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn test_last_modified(){
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::IF_RANGE, "Wed, 18 Feb 2015 23:16:09 GMT");

        let value = headers.get_typed_headers().if_range().unwrap();

        assert_eq!(
            &DateTimeOffset::from(FixedOffset::east(0).ymd(2015, 2, 18).and_hms(23, 16, 9)),
                   value.last_modified().unwrap());

        assert_eq!("Wed, 18 Feb 2015 23:16:09 GMT", value.to_string())
    }

    #[test]
    fn test_etag(){
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::IF_RANGE, r#""675af34563dc-tr34""#);

        let value = headers.get_typed_headers().if_range().unwrap();
        assert_eq!(r#""675af34563dc-tr34""#, value.to_string());
    }

    #[test]
    fn test_etag1(){
        let mut headers = HeaderMap::new();
        headers.add_raw(header_names::IF_RANGE, r#"W/"675af34563dc-tr34""#);

        let value = headers.get_typed_headers().if_range().unwrap();
        assert_eq!(r#"W/"675af34563dc-tr34""#, value.to_string())
    }
}