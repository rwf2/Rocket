
use super::{http_rule_parser, header_utilities};

/// Represents a byte range in a Range header value.
#[derive(Default, Clone, Debug)]
pub struct RangeItemHeaderValue {
    pub from: Option<u64>,
    pub to: Option<u64>
}

impl RangeItemHeaderValue {
    pub fn new(from: Option<u64>, to: Option<u64>) -> Self {
        Self{ from, to }
    }
    pub fn normalize(&self, len: u64) -> Option<Self> {
        let mut start = self.from;
        let mut end = self.to;
        // X-[Y]
        if let Some(s) = start {
            if s >= len {
                // Not satisfiable, skip/discard.
                return None;
            }
            if end.is_none() || end.unwrap() >= len {
                end = Some(len - 1);
            }
        } else {
            if let Some(e) = end {
                if e == 0 {
                    // Not satisfiable, skip/discard.
                    return None;
                }
            }
            let bytes = std::cmp::min(end.unwrap(), len);
            start = Some(len - bytes);
            end = Some(start.unwrap() + bytes - 1);
        }
        Some(Self::new(start, end))
    }
    //noinspection RsSelfConvention
    pub(super) fn get_range_item_length(input: &[char], start_index: usize, parsed_value: &mut Self) -> usize {
        // This parser parses number ranges: e.g. '1-2', '1-', '-2'.
        if input.len() == 0 || input[0] ==' ' || start_index >= input.len() {
            return 0;
        }
        // Caller must remove leading whitespaces. If not, we'll return 0.
        let mut current = start_index;
        // Try parse the first value of a value pair.
        let from_start_index = current;
        let from_len = http_rule_parser::get_number_length(input, current, false);
        if from_len > http_rule_parser::MAX_INT64DIGITS {
            return 0;
        }
        current += from_len;
        current += http_rule_parser::get_whitespace_length(input, current);

        if current == input.len() || input[current] != '-' {
            // We need a '-' character otherwise this can't be a valid range.
            return 0;
        }

        current += 1; // skip the '-' character
        current += http_rule_parser::get_whitespace_length(input, current);

        let to_start_index = current;
        let mut to_len = 0;

        if current < input.len() {
            to_len = http_rule_parser::get_number_length(input, current, false);
            if to_len > http_rule_parser::MAX_INT64DIGITS {
                return 0;
            }
            current += to_len;
            current += http_rule_parser::get_whitespace_length(input, current);
        }
        if from_len == 0 && to_len == 0 {
            return 0;  // At least one value must be provided in order to be a valid range.
        }

        // Try convert first value to int64
        let mut from= 0;
        if from_len > 0 && !header_utilities::try_parse_non_negative_int64(&input[from_start_index..from_start_index + from_len], &mut from) {
            return 0;
        }
        let mut to= 0;

        // Try convert first value to int64
        if to_len > 0 && !header_utilities::try_parse_non_negative_int64(&input[to_start_index..to_start_index + to_len], &mut to) {
            return 0;
        }

        // 'from' must not be greater than 'to'
        if from_len > 0 && to_len > 0 && from > to {
            return 0;
        }
        *parsed_value = Self::new(
            if from_len == 0 { None } else {Some(from) },
            if to_len == 0 { None } else { Some(to) }
        );
        current - start_index
    }
    //noinspection RsSelfConvention
    pub(super) fn get_range_item_list_length(input: &[char], start_index: usize, ranges: &mut Vec<Self>) -> usize {

        if input.len() == 0 || start_index >=input.len() {
            return 0;
        }
        // Empty segments are allowed, so skip all delimiter-only segments (e.g. ", ,").
        let mut separator_found = false;
        let mut current = header_utilities::get_next_non_empty_or_whitespace_index(input, start_index, true, &mut separator_found);
        // It's OK if we didn't find leading separator characters. Ignore 'separatorFound'.

        if current == input.len() {
            return 0;
        }
        let mut range = Self::default();

        loop {
            let range_len  = Self::get_range_item_length(input, current, &mut range);
            if range_len == 0 {
                return 0;
            }
            ranges.push(range.clone());
            current += range_len;
            current = header_utilities::get_next_non_empty_or_whitespace_index(input, current, true, &mut separator_found);

            // If the string is not consumed, we must have a delimiter, otherwise the string is not a valid
            // range list.
            if current < input.len() && !separator_found {
                return 0;
            }
            if current == input.len() {
                return current - start_index;
            }
        }
    }
}