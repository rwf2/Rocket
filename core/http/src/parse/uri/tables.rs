use percent_encoding::AsciiSet;

// Generates an AsciiSet from a full character table:
//
// table_to_ascci_set!( ExistingAsciiSet, 0, [ 0, 1, b'x', ... ] );
//
// A 0 or 1 token in the ith position indicates that the ASCII character i
// should be included in the set. Any other literal in the ith position
// indicates that the ASCII character with value i should not be included in the
// set.
//
// The table's last index must be 127 or earlier. All values in the original
// set, up to the end of the passed-in table, are overwritten.
macro_rules! table_to_ascii_set {
    ($base:expr, $i:expr, [ 0,           $($rest:tt,)* ]) => { table_to_ascii_set!($base.add($i),    $i+1, [ $($rest,)* ]); };
    ($base:expr, $i:expr, [ 1,           $($rest:tt,)* ]) => { table_to_ascii_set!($base.add($i),    $i+1, [ $($rest,)* ]); };
    ($base:expr, $i:expr, [ $ch:literal, $($rest:tt,)* ]) => { table_to_ascii_set!($base.remove($i), $i+1, [ $($rest,)* ]); };
    ($base:expr, $i:expr, [ ]) => { $base };
}

// Generates an AsciiSet accompanying a character table. This is
// used to keep `PATH_CHARS` in sync with `PATH_SET`.
//
// The first block is limited to 128 entries, since it is passed
// to table_to_ascii_set!
macro_rules! table_and_asciiset {
    (
        const $name:ident: [u8; $size:expr] = [ $($block1:tt)* ] [ $($block2:tt)* ];
        pub const $setname:ident: AsciiSet;
    ) => {
        const $name: [u8; $size] = [ $($block1)* $($block2)* ] ;
        pub const $setname: AsciiSet = table_to_ascii_set!(percent_encoding::CONTROLS, 0, [ $($block1)* ]);
    };
}

table_and_asciiset! {
const PATH_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,  b'!',     0,     0,  b'$',  b'%',  b'&', b'\'', //  3x
     b'(',  b')',  b'*',  b'+',  b',',  b'-',  b'.',  b'/',  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',  b':',  b';', //  5x
     // <             > (1 used to indicate these are valid in route URIs only)
        1,  b'=',     1,     0,  b'@',  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z',     0,     0,     0,     0,  b'_',     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,     0,     0,  b'~',     0, ] [ 0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0,                             // 25x
];
pub const PATH_SET: AsciiSet;
}

#[inline(always)]
pub fn is_pchar(c: u8) -> bool {
    PATH_CHARS[c as usize] == c
}

#[inline(always)]
pub fn is_pchar_or_rchar(c: u8) -> bool {
    PATH_CHARS[c as usize] != 0
}

const REG_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,  b'!',     0,     0,  b'$',     0,  b'&', b'\'', //  3x
     b'(',  b')',  b'*',  b'+',  b',',  b'-',  b'.',     0,  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',     0,  b';', //  5x
        0,  b'=',     0,     0,     0,  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z',     0,     0,     0,     0,  b'_',     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,     0,     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

#[inline(always)]
pub fn is_reg_name_char(c: u8) -> bool {
    REG_CHARS[c as usize] != 0
}

#[cfg(test)]
mod tests {
    fn test_char_table(table: &[u8]) {
        for (i, &v) in table.iter().enumerate() {
            if v != 0 && v != 1 {
                assert_eq!(i, v as usize);
            }
        }
    }

    #[test]
    fn check_tables() {
        test_char_table(&super::PATH_CHARS[..]);
        test_char_table(&super::REG_CHARS[..]);
    }
}
