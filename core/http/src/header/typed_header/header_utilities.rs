use super::http_rule_parser;


pub const BYTES_UNIT: &'static str = "bytes";

pub fn try_parse_non_negative_int64(value: &[char], result: &mut u64) -> bool {
    *result = 0;
    if value.is_empty() || value[0] == ' '{
        return false;
    }

    let mut digit;
    for ch in value {
        digit = *ch as u64 - 0x30;
        if digit > 9 {
            *result = 0;
            return false;
        }

        if let Some(r) = result.checked_mul(10)
            .map(|n| n.checked_add(digit))
            .unwrap_or(None) {
            *result = r
        } else {
            *result = 0;
            return false;
        }
    }
    true
}

pub fn get_next_non_empty_or_whitespace_index(input: &[char], start_index: usize, skip_empty_values: bool, separator_found: &mut bool) -> usize {

    *separator_found = false;

    let mut current = start_index + http_rule_parser::get_whitespace_length(input, start_index);
    if current == input.len() || input[current] != ',' {
        return current;
    }

    // If we have a separator, skip the separator and all following whitespaces. If we support
    // empty values, continue until the current character is neither a separator nor a whitespace.
    *separator_found = true;

    current += 1;  // skip delimiter.
    current += http_rule_parser::get_whitespace_length(input, current);

    if skip_empty_values {
        while current < input.len() && input[current] == ',' {
            current += 1;  // skip delimiter.
            current += http_rule_parser::get_whitespace_length(input, current);

        }
    }
    return current;
}