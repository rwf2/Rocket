
use super::{
    http_rule_parser
};
use super::http_rule_parser::HttpParseResult;
use std::convert::TryFrom;


/// Represents an entity-tag (*etag*) header value.
#[derive(Debug, Eq, PartialEq)]
pub struct EntityTagHeaderValue {
    tag: Vec<char>,
    is_weak: bool
}

impl EntityTagHeaderValue {

    /// Used by the parser to create a new instance of this type.
    fn new() -> Self {
        Self { tag: Vec::new(), is_weak: false }
    }
    pub fn any() -> Self {
        Self{ tag: vec!['*'], is_weak: false }
    }
    pub fn compare(&self, other: &Self, use_strong_comparison: bool) -> bool {
        if use_strong_comparison {
            !self.is_weak && !other.is_weak && self.tag == other.tag
        } else {
            self.tag == other.tag
        }
    }

    //noinspection ALL
    pub(super) fn get_entity_tag_length(input: &[char], start_index: usize, parsed_value: &mut Self) -> usize {
        if input.is_empty() || start_index >= input.len() {
            return 0;
        }
        // Caller must remove leading whitespaces. If not, we'll return 0.
        let mut is_weak = false;
        let mut current = start_index;
        let first_char = input[start_index];
        if first_char == '*' {
            // We have '*' value, indicating "any" ETag.
            *parsed_value = Self::any();
            current += 1;
        } else {
            // The RFC defines 'W/' as prefix, but we'll be flexible and also accept lower-case 'w'.
            if first_char == 'W' || first_char == 'w' {
                current += 1;
                // We need at least 3 more chars: the '/' character followed by two quotes.
                if current + 2 >= input.len() || input[current] != '/' {
                    return 0;
                }
                is_weak = true;
                current += 1;
                current = current + http_rule_parser::get_whitespace_length(input, current);
            }
            // let tag_start_index = current;
            let mut tag_len = 0;
            if http_rule_parser::get_quoted_string_length(input, current, &mut tag_len)
                != HttpParseResult::Parsed {
                return 0;
            }

            *parsed_value = Self::new();
            if tag_len == input.len() {
                parsed_value.tag = input.to_vec();
                parsed_value.is_weak = false;
            } else {
                parsed_value.tag = input[start_index .. start_index + tag_len + 2].to_vec();
                parsed_value.is_weak = is_weak;
            }

            current += tag_len;
        }
        current += http_rule_parser::get_whitespace_length(input, current);

        current - start_index
    }
}

impl From<String> for EntityTagHeaderValue {
    fn from(mut s: String) -> Self {
        if !s.starts_with('"') {
            s.insert(0, '"');
        }
        if !s.ends_with('"') {
            s.push('"');
        }
        Self{
            tag: s.chars().collect(),
            is_weak: false
        }
    }
}

impl TryFrom<Vec<&str>> for EntityTagHeaderValue {
    type Error = ();

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }

        let input: Vec<char> = value[0].chars().collect();

        let mut v = Self::new();

        if Self::get_entity_tag_length(&input, 0, &mut v) == 0 {
            return Err(());
        }
        Ok(v)
    }
}


impl ToString for EntityTagHeaderValue {
    fn to_string(&self) -> String {
        self.tag.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_from_str() {
        let s = String::from("123456789");
        let etag = EntityTagHeaderValue::from(s);

        assert_eq!(r#""123456789""#, etag.to_string().as_str())
    }
}
