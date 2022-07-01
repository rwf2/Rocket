use std::collections::HashMap;

use crate::form::*;

fn parse<'v, T: FromForm<'v>>(values: &[&'v str]) -> Result<'v, T> {
    Form::parse_iter(values.iter().cloned().map(ValueField::parse))
}

macro_rules! map {
    ($($key:expr => $value:expr),* $(,)?) => ({
        let mut map = std::collections::HashMap::new();
        $(map.insert($key.into(), $value.into());)*
        map
    });
}

macro_rules! vec {
    ($($value:expr),* $(,)?) => ({
        let mut vec = Vec::new();
        $(vec.push($value.into());)*
        vec
    });
}

macro_rules! assert_values_parse_eq {
    ($($v:expr => $T:ty = $expected:expr),* $(,)?) => (
        $(
            assert_value_parse_eq!($v as &[&str] => $T = $expected);
        )*
    )
}

macro_rules! assert_value_parse_eq {
    ($v:expr => $T:ty = $expected:expr) => (
        let expected: $T = $expected;
        match parse::<$T>($v) {
            Ok(actual) if actual == expected => { /* ok */ },
            Ok(actual) => {
                panic!("unexpected parse of {:?} ({:?} instead of {:?})",
                    $v, actual, expected)
            }
            Err(e) => panic!("parse `{:?} {}` failed: {:?}", $v, stringify!(=> $T = $expected), e)
        }
    )
}

macro_rules! assert_parses_fail {
    ($($v:expr => $T:ty),* $(,)?) => (
        $(
            assert_parse_fails!($v as &[&str] => $T);
        )*
    )
}

macro_rules! assert_parse_fails {
    ($v:expr => $T:ty) => (
        let diag = format!("{:?} {}", $v, stringify!(=> $T));
        match parse::<$T>($v) {
            Ok(actual) => panic!("unexpectedly parsed {} as {:?}", diag, actual),
            Err(_) => { /* ok */ }
        }
    )
}

#[test]
fn time() {
    use time::{macros::{date, time}, Date, Time, PrimitiveDateTime as DateTime};

    assert_values_parse_eq! {
        &["=2010-10-20"] => Date = date!(2010-10-20),
        &["=2012-01-20"] => Date = date!(2012-01-20),
        &["=2020-01-20T02:30"] => DateTime = DateTime::new(date!(2020-01-20), time!(2:30)),
        &["=2020-01-01T02:30:12"] => DateTime = DateTime::new(date!(2020-01-01), time!(2:30:12)),
        &["=20:20:52"] => Time = time!(20:20:52),
        &["=06:08"] => Time = time!(06:08),
    }
}

#[test]
fn bool() {
    assert_values_parse_eq! {
        &["=true", "=yes", "=on", ""] => Vec<bool> = vec![true, true, true, true],
        &["=false", "=no", "=off"] => Vec<bool> = vec![false, false, false],
        &["=tRuE", "=YES", "=On"] => Vec<bool> = vec![true, true, true],
        &["=fAlSE", "=NO", "=OFF"] => Vec<bool> = vec![false, false, false],
    }

    assert_parses_fail! {
        &[] => Strict<bool>,
        &["=unknown"] => bool,
        &["=unknown", "=please"] => Vec<bool>,
    }
}

#[test]
fn defaults() {
    assert_values_parse_eq! {
        &[] => bool = false,
        &[] => Option<&str> = None,
        &[] => Option<time::Date> = None,

        &[] => Option<bool> = None,
        &[] => Option<Strict<bool>> = None,

        &[] => Result<'_, bool> = Ok(false),
        &[] => Result<'_, Strict<bool>> = Err(error::ErrorKind::Missing.into()),

        &["=unknown"] => Option<bool> = None,
        &["=unknown"] => Option<Strict<bool>> = None,
        &["=unknown"] => Option<Lenient<bool>> = None,

        &[] => Option<Lenient<bool>> = Some(false.into()),
        &["=123"] => Option<time::Date> = None,

        &["=no"] => Option<bool> = Some(false),
        &["=yes"] => Option<bool> = Some(true),
        &["=yes"] => Option<Lenient<bool>> = Some(true.into()),
        &["=yes"] => Option<Strict<bool>> = Some(true.into()),
    }
}

#[test]
fn potpourri() {
    assert_values_parse_eq! {
        &["a.b=10"] => usize = 10,
        &["a=10"] => u8 = 10,
        &["=10"] => u8 = 10,
        &["=5", "=3", "=4"] => Vec<&str> = vec!["5", "3", "4"],
        &["=5", "=3", "=4"] => Vec<&str> = vec!["5", "3", "4"],
        &["a=3", "b=4", "c=5"] => Vec<u8> = vec![3, 4, 5],
        &["=3", "=4", "=5"] => Vec<u8> = vec![3, 4, 5],
        &["=3", "=4", "=5"] => Vec<Vec<u8>> = vec![vec![3], vec![4], vec![5]],
        &["[]=3", "[]=4", "[]=5"] => Vec<Vec<u8>> = vec![vec![3], vec![4], vec![5]],
        &["[][]=3", "[][]=4", "[][]=5"] => Vec<Vec<u8>> = vec![vec![3], vec![4], vec![5]],
        &["[]=5", "[]=3", "[]=4"] => Vec<&str> = vec!["5", "3", "4"],
        &["[0]=5", "[0]=3", "=4", "=6"] => Vec<Vec<u8>>
            = vec![vec![5, 3], vec![4], vec![6]],
        &[".0=5", ".1=3"] => (u8, usize) = (5, 3),
        &["0=5", "1=3"] => (u8, usize) = (5, 3),
        &["[bob]=Robert", ".j=Jack", "s=Stan", "[s]=Steve"] => HashMap<&str, &str>
            = map!["bob" => "Robert", "j" => "Jack", "s" => "Stan"],
        &["[bob]=Robert", ".j=Jack", "s=Stan", "[s]=Steve"]
            => HashMap<&str, Vec<&str>>
            = map![
                "bob" => vec!["Robert"],
                "j" => vec!["Jack"],
                "s" => vec!["Stan", "Steve"]
            ],
        &["[k:0]=5", "[k:0]=3", "[v:0]=20", "[56]=2"] => HashMap<Vec<&str>, usize>
            = map![vec!["5", "3"] => 20u8, vec!["56"] => 2u8],
        &["[k:0]=5", "[k:0]=3", "[0]=20", "[56]=2"] => HashMap<Vec<&str>, usize>
            = map![vec!["5", "3"] => 20u8, vec!["56"] => 2u8],
        &[
            "[k:a]0=5", "[a]=hi", "[v:b][0]=10", "[k:b].0=1",
            "[k:b].1=hi", "[a]=hey", "[k:a]1=3"
        ] => HashMap<(usize, &str), Vec<&str>>
            = map![
                (5, "3".into()) => vec!["hi", "hey"],
                (1, "hi".into()) => vec!["10"]
            ],
        &[
            "[0][hi]=10", "[0][hey]=12", "[1][bob]=0", "[1].blam=58", "[].0=1",
            "[].whoops=999",
        ] => Vec<HashMap<&str, usize>>
            = vec![
                map!["hi" => 10u8, "hey" => 12u8],
                map!["bob" => 0u8, "blam" => 58u8],
                map!["0" => 1u8],
                map!["whoops" => 999usize]
            ],
    }
}
