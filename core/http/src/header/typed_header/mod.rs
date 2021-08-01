use std::convert::TryFrom;
use super::{HeaderMap, Header};


pub mod header_names;
mod header_utilities;
mod http_rule_parser;


mod range_header_value;
mod range_item_header_value;
mod range_condition_header_value;
mod entity_tag_header_value;
mod date_time_offset;
mod content_range_header_value;


pub use range_header_value::RangeHeaderValue;
pub use range_item_header_value::RangeItemHeaderValue;
pub use range_condition_header_value::RangeConditionHeaderValue;
pub use entity_tag_header_value::EntityTagHeaderValue;
pub use content_range_header_value::ContentRangeHeaderValue;
pub use date_time_offset::DateTimeOffset;

/// Strongly typed HTTP request headers.
pub struct RequestHeaders<'r, 'h> (&'r HeaderMap<'h>);

impl<'r, 'h> RequestHeaders<'r, 'h> {

    /// Get typed HeaderValue
    pub fn range(&self) -> Option<RangeHeaderValue> {
        self.0.get_typed_header_value(header_names::RANGE)
    }

    /// Get typed HeaderValue
    pub fn if_range(&self) -> Option<RangeConditionHeaderValue> {
        self.0.get_typed_header_value(header_names::IF_RANGE)
    }

    /// Get typed HeaderValue
    pub fn if_modified_since(&self) -> Option<DateTimeOffset> {
        self.0.get_typed_header_value(header_names::IF_MODIFIED_SINCE)
    }

    /// Get typed HeaderValue
    pub fn if_unmodified_since(&self) -> Option<DateTimeOffset> {
        self.0.get_typed_header_value(header_names::IF_UNMODIFIED_SINCE)
    }

    /// Get typed HeaderValue
    pub fn if_match(&self) -> Vec<EntityTagHeaderValue> {
        self.0.get_typed_header_value_list(header_names::IF_MATCH)
    }

    /// Get typed HeaderValue
    pub fn if_none_match(&self) -> Vec<EntityTagHeaderValue> {
        self.0.get_typed_header_value_list(header_names::IF_NONE_MATCH)
    }
}

/// Strongly typed HTTP request headers.
pub trait TypedHeaders {
    /// Gets strongly typed HTTP request headers.
    fn get_typed_headers(&self) -> RequestHeaders<'_, '_>;

    /// Gets strongly typed HTTP request header value.
    fn get_typed_header_value<'a, T>(&'a self, name: &str) -> Option<T> where T: TryFrom<Vec<&'a str>>;

    /// Gets strongly typed HTTP request header values.
    fn get_typed_header_value_list<'a, T>(&'a self, name: &str) -> Vec<T> where T: TryFrom<Vec<&'a str>>;
}

impl<'h> TypedHeaders for HeaderMap<'h> {
    fn get_typed_headers(&self) -> RequestHeaders<'_, '_> {
        RequestHeaders (self)
    }

    fn get_typed_header_value<'a, T>(&'a self, name: &str) -> Option<T> where T: TryFrom<Vec<&'a str>> {
        let header_values: Vec<&str> = self.get(name).collect();
        let p = T::try_from(header_values)
            .map(|x| Some(x))
            .unwrap_or(None);

        p
    }

    fn get_typed_header_value_list<'a, T>(&'a self, name: &str) -> Vec<T> where T: TryFrom<Vec<&'a str>> {
        let header_values: Vec<&str> = self.get(name).collect();
        let mut v = Vec::new();
        for header_value in header_values {
            if let Ok(t) = T::try_from(vec![header_value]) {
                v.push(t)
            }
        }
        v
    }
}