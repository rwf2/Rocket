/// Takes a set of sets of byte characters, return a 2^8 array with non-zero
/// values at the indices corresponding to the character byte values.
const fn char_table(sets: &[&[u8]]) -> [u8; 256] {
    let mut table = [0u8; 256];

    let mut i = 0;
    while i < sets.len() {
        let set: &[u8] = sets[i];

        let mut j = 0;
        while j < set.len() {
            let c: u8 = set[j];
            table[c as usize] = c;
            j += 1;
        }

        i += 1;
    }

    table
}

const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

const DIGIT: &[u8] = b"0123456789";

const PCT_ENCODED: &[u8] = b"%ABCDEFabcdef0123456789";

const SUB_DELIMS: &[u8] = b"!$&'()*+,;=";

const SCHEME_CHAR: [u8; 256] = char_table(&[ALPHA, DIGIT, b"+-."]);

const UNRESERVED: [u8; 256] = char_table(&[ALPHA, DIGIT, b"-._~"]);

const REG_NAME_CHARS: [u8; 256] = char_table(&[&UNRESERVED, PCT_ENCODED, SUB_DELIMS]);

const USER_INFO_CHARS: [u8; 256] = char_table(&[&REG_NAME_CHARS, b":"]);

pub const PATH_CHARS: [u8; 256] = char_table(&[
    &REG_NAME_CHARS,
    b":@/",
    // NOTE: these are _not_ accepted in RFC 7230/3986. However, browsers
    // routinely send these unencoded, so allow them to support the real-world.
    b"[]",
]);

const QUERY_CHARS: [u8; 256] = char_table(&[
    &PATH_CHARS,
    b"/?",
    // NOTE: these are _not_ accepted in RFC 7230/3986. However, browsers
    // routinely send these unencoded, so allow them to support the real-world.
    b"{}[]\\^`|",
]);

#[inline(always)]
pub const fn is_pchar(&c: &u8) -> bool {
    PATH_CHARS[c as usize] != 0
}

#[inline(always)]
pub const fn is_host_char(c: &u8) -> bool {
    is_pchar(c) && *c != b'[' && *c != b']'
}

#[inline(always)]
pub const fn is_scheme_char(&c: &u8) -> bool {
    SCHEME_CHAR[c as usize] != 0
}

#[inline(always)]
pub const fn is_user_info_char(&c: &u8) -> bool {
    USER_INFO_CHARS[c as usize] != 0
}

#[inline(always)]
pub const fn is_qchar(&c: &u8) -> bool {
    QUERY_CHARS[c as usize] != 0
}

#[inline(always)]
pub const fn is_reg_name_char(&c: &u8) -> bool {
    REG_NAME_CHARS[c as usize] != 0
}

#[cfg(test)]
mod tests {
    fn test_char_table(table: &[u8]) {
        for (i, &v) in table.iter().enumerate() {
            if v != 0 {
                assert_eq!(i, v as usize);
            }
        }
    }

    #[test]
    fn check_tables() {
        test_char_table(&super::PATH_CHARS[..]);
        test_char_table(&super::QUERY_CHARS[..]);
        test_char_table(&super::REG_NAME_CHARS[..]);
    }
}
