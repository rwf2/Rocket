#[inline(always)]
pub fn is_whitespace(&byte: &char) -> bool {
    byte == ' ' || byte == '\t'
}

#[inline]
pub fn is_valid_token(&c: &char) -> bool {
     matches!(c, '0'..='9' | 'A'..='Z' | '^'..='~' | '#'..='\''
            | '!' | '*' | '+' | '-' | '.')
    }
