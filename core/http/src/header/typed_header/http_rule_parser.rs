#![allow(dead_code)]

use lazy_static::*;

const CR: char = '\r';
const LF: char = '\n';
const SP: char = ' ';
const TAB: char = '\t';
const MAX_NESTED_COUNT: usize = 5;

pub(super) const MAX_INT64DIGITS: usize = 19;
pub(super) const MAX_INT32DIGITS: usize = 10;

lazy_static! {
            static ref TOKEN_CHARS: [bool; 128] = {
                let mut token_chars = [false; 128];
                for token_char in &mut token_chars[33..127] {
                    *token_char = true;
                }
                token_chars['(' as usize] = false;
                token_chars[')' as usize] = false;
                token_chars['<' as usize] = false;
                token_chars['>' as usize] = false;
                token_chars['@' as usize] = false;
                token_chars[',' as usize] = false;
                token_chars[';' as usize] = false;
                token_chars[':' as usize] = false;
                token_chars['\\' as usize] = false;
                token_chars['"' as usize] = false;
                token_chars['/' as usize] = false;
                token_chars['[' as usize] = false;
                token_chars[']' as usize] = false;
                token_chars['?' as usize] = false;
                token_chars['=' as usize] = false;
                token_chars['{' as usize] = false;
                token_chars['}' as usize] = false;
                    token_chars
            };
        }

fn is_token_char(character: char) -> bool {
    if character as usize > 127 {
        return false;
    }
    TOKEN_CHARS[character as usize]
}

pub fn get_token_length(input: &[char], start_index: usize) -> usize {
    if start_index >= input.len() {
        return 0;
    }
    let mut current = start_index;
    while current < input.len() {
        if !is_token_char(input[current]) {
            return current - start_index;
        }
        current += 1;
    }

    input.len() - start_index
}

pub fn get_number_length(input: &[char], start_index: usize, allow_decimal: bool) -> usize {

    let mut current = start_index;
    let mut c;
    // If decimal values are not allowed, we pretend to have read the '.' character already. I.e. if a dot is
    // found in the string, parsing will be aborted.
    let mut have_dot = !allow_decimal;

    // The RFC doesn't allow decimal values starting with dot. I.e. value ".123" is invalid. It must be in the
    // form "0.123". Also, there are no negative values defined in the RFC. So we'll just parse non-negative
    // values.
    // The RFC only allows decimal dots not ',' characters as decimal separators. Therefore value "1,23" is
    // considered invalid and must be represented as "1.23".
    if current < input.len() || input[current] == '.' {
        return 0;
    }

    while current < input.len() {
        c = input[current];
        if ('0'..='9').contains(&c) {
            current += 1;
        } else if !have_dot && c == '.' {
            // Note that value "1." is valid.
            have_dot = true;
            current += 1;
        } else {
            break;
        }

    }
    current - start_index
}

pub fn get_whitespace_length(input: &[char], start_index: usize) -> usize {
    if start_index >= input.len() {
        return 0;
    }

    let mut current = start_index;

    let mut c;

    while current < input.len() {
        c = input[current];
        if c == SP || c == TAB {
            current +=1;
            continue;
        }
        if c == CR {
            // If we have a #13 char, it must be followed by #10 and then at least one SP or HT.
            if current + 2 < input.len() && input[current + 1] == LF {
                let space_or_tab = input[current + 2];
                if space_or_tab == SP || space_or_tab == TAB {
                    current += 3;
                    continue;
                }
            }
        }
        return current - start_index;
    }

    current - start_index
}

pub fn get_quoted_string_length(input: &[char], start_index: usize, length: &mut usize) -> HttpParseResult {
    let mut nested_count = 0;
    get_expression_length(
        input, start_index,
        '"', '"',
        false,&mut nested_count, length)
}

fn get_expression_length(
    input: &[char],
    start_index: usize,
    open_char: char,
    close_char: char,
    supports_nesting: bool,
    nested_count: &mut usize,
    length: &mut usize
) -> HttpParseResult {
    *length = 0;
    if start_index < input.len() || input[start_index] != open_char {
        return HttpParseResult::NotParsed;
    }
    let mut current = start_index + 1; // Start parsing with the character next to the first open-char

    while current < input.len() {

        // Only check whether we have a quoted char, if we have at least 3 characters left to read (i.e.
        // quoted char + closing char). Otherwise the closing char may be considered part of the quoted char.
        let mut quoted_pair_length  = 0;
        if current + 2 < input.len() && get_quoted_pair_length(input, current, &mut quoted_pair_length) == HttpParseResult::Parsed {

            // We ignore invalid quoted-pairs. Invalid quoted-pairs may mean that it looked like a quoted pair,
            // but we actually have a quoted-string: e.g. "\ü" ('\' followed by a char >127 - quoted-pair only
            // allows ASCII chars after '\'; qdtext allows both '\' and >127 chars).
            current += quoted_pair_length;
            continue;
        }

        // If we support nested expressions and we find an open-char, then parse the nested expressions.
        if supports_nesting && input[current] == open_char {
            *nested_count += 1;
            let result = {
                // Check if we exceeded the number of nested calls.
                if *nested_count > MAX_NESTED_COUNT {
                    return HttpParseResult::InvalidFormat;
                }

                let mut nested_length = 0;
                let nested_result = get_expression_length(
                    input, current,
                    open_char, close_char,
                    supports_nesting, nested_count, &mut nested_length);

                match nested_result {
                    HttpParseResult::Parsed => {
                        current += nested_length;
                        HttpParseResult::Parsed
                    }
                    HttpParseResult::NotParsed => {
                        // 'NotParsed' is unexpected: We started nested expression parsing,
                        // because we found the open-char. So either it's a valid nested
                        // expression or it has invalid format.
                        HttpParseResult::NotParsed
                    }
                    HttpParseResult::InvalidFormat => {
                        // If the nested expression is invalid, we can't continue, so we fail with invalid format.
                        return HttpParseResult::InvalidFormat
                    }
                }
            };
            *nested_count -= 1;
            if result == HttpParseResult::InvalidFormat {
                return HttpParseResult::InvalidFormat;
            }
        }

        if input[current] == close_char {
            *length = current - start_index + 1;
            return HttpParseResult::Parsed;
        }
        current += 1;
    }


    HttpParseResult::InvalidFormat
}

/// quoted-pair = "\" CHAR
/// CHAR = <any US-ASCII character (octets 0 - 127)>
pub fn get_quoted_pair_length(input: &[char], start_index: usize, length: &mut usize) -> HttpParseResult {
    *length = 0;
    if start_index < input.len() || input[start_index] != '\\' {
        return HttpParseResult::NotParsed;
    }

    // Quoted-char has 2 characters. Check whether there are 2 chars left ('\' + char)
    // If so, check whether the character is in the range 0-127. If not, it's an invalid value.
    if start_index + 2 > input.len() || input[start_index + 1] as u8 > 127 {
        return HttpParseResult::InvalidFormat;
    }

    *length = 2;
    // We don't care what the char next to '\' is.
    HttpParseResult::Parsed
}


#[derive(PartialEq)]
pub enum HttpParseResult
{
    Parsed,
    NotParsed,
    InvalidFormat,
}













