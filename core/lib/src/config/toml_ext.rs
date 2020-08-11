use std::fmt;
use std::result::Result as StdResult;

use crate::config::Value;

use pear::macros::{parse, parser, switch};
use pear::parsers::*;
use pear::combinators::*;

type Input<'a> = pear::input::Pear<&'a str>;
type Result<'a, T> = pear::input::Result<T, Input<'a>>;

#[inline(always)]
pub fn is_whitespace(&byte: &char) -> bool {
    byte.is_ascii_whitespace()
}

#[inline(always)]
fn is_not_separator(&byte: &char) -> bool {
    match byte {
        ',' | '{' | '}' | '[' | ']' => false,
        _ => true
    }
}

// FIXME: Be more permissive here?
#[inline(always)]
fn is_ident_char(&byte: &char) -> bool {
    byte.is_ascii_alphanumeric() || byte == '_' || byte == '-'
}

#[parser]
fn array<'a>(input: &mut Input<'a>) -> Result<'a, Value> {
    Value::Array(delimited_collect('[', value, ',', ']')?)
}

#[parser]
fn key<'a>(input: &mut Input<'a>) -> Result<'a, String> {
    take_some_while(is_ident_char)?.to_string()
}

#[parser]
fn key_value<'a>(input: &mut Input<'a>) -> Result<'a, (String, Value)> {
    let key = (surrounded(key, is_whitespace)?, eat('=')?).0.to_string();
    (key, surrounded(value, is_whitespace)?)
}

#[parser]
fn table<'a>(input: &mut Input<'a>) -> Result<'a, Value> {
    Value::Table(delimited_collect('{', key_value, ',', '}')?)
}

#[parser]
fn value<'a>(input: &mut Input<'a>) -> Result<'a, Value> {
    skip_while(is_whitespace)?;
    let val = switch! {
        eat_slice("true") => Value::Boolean(true),
        eat_slice("false") => Value::Boolean(false),
        peek('{') => table()?,
        peek('[') => array()?,
        peek('"') => Value::String(delimited('"', |_| true, '"')?.to_string()),
        _ => {
            let value_str = take_some_while(is_not_separator)?;
            if let Ok(int) = value_str.parse::<i64>() {
                Value::Integer(int)
            } else if let Ok(float) = value_str.parse::<f64>() {
                Value::Float(float)
            } else {
                Value::String(value_str.into())
            }
        }
    };

    skip_while(is_whitespace)?;
    val
}

pub fn parse_simple_toml_value(input: &str) -> StdResult<Value, String> {
    parse!(value: input).map_err(|e| e.to_string())
}

/// A simple wrapper over a `Value` reference with a custom implementation of
/// `Display`. This is used to log config values at initialization.
pub struct LoggedValue<'a>(pub &'a Value);

impl fmt::Display for LoggedValue<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::config::Value::*;
        match *self.0 {
            String(_) | Integer(_) | Float(_) | Boolean(_) | Datetime(_) | Array(_) => {
                self.0.fmt(f)
            }
            Table(ref map) => {
                write!(f, "{{ ")?;
                for (i, (key, val)) in map.iter().enumerate() {
                    write!(f, "{} = {}", key, LoggedValue(val))?;
                    if i != map.len() - 1 { write!(f, ", ")?; }
                }

                write!(f, " }}")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use toml::map::Map;

    use super::parse_simple_toml_value;
    use super::Value::{self, *};

    macro_rules! assert_parse {
        ($string:expr, $value:expr) => (
            match parse_simple_toml_value($string) {
                Ok(value) => assert_eq!(value, $value),
                Err(e) => panic!("{:?} failed to parse: {:?}", $string, e)
            };
        )
    }

    #[test]
    fn parse_toml_values() {
        assert_parse!("1", Integer(1));
        assert_parse!("1.32", Float(1.32));
        assert_parse!("true", Boolean(true));
        assert_parse!("false", Boolean(false));
        assert_parse!("\"hello, WORLD!\"", String("hello, WORLD!".into()));
        assert_parse!("hi", String("hi".into()));
        assert_parse!("\"hi\"", String("hi".into()));

        assert_parse!("[]", Array(Vec::new()));
        assert_parse!("[1]", vec![1].into());
        assert_parse!("[1, 2, 3]", vec![1, 2, 3].into());
        assert_parse!("[1.32, 2]", Array(vec![1.32.into(), 2.into()]));

        assert_parse!("{}", Table(Map::new()));

        assert_parse!("{a=b}", Table({
            let mut map = Map::new();
            map.insert("a".into(), "b".into());
            map
        }));

        assert_parse!("{v=1, on=true,pi=3.14}", Table({
            let mut map = Map::new();
            map.insert("v".into(), 1.into());
            map.insert("on".into(), true.into());
            map.insert("pi".into(), 3.14.into());
            map
        }));

        assert_parse!("{v=[1, 2, 3], v2=[a, \"b\"], on=true,pi=3.14}", Table({
            let mut map = Map::new();
            map.insert("v".into(), vec![1, 2, 3].into());
            map.insert("v2".into(), vec!["a", "b"].into());
            map.insert("on".into(), true.into());
            map.insert("pi".into(), 3.14.into());
            map
        }));

        assert_parse!("{v=[[1], [2, 3], [4,5]]}", Table({
            let mut map = Map::new();
            let first: Value = vec![1].into();
            let second: Value = vec![2, 3].into();
            let third: Value = vec![4, 5].into();
            map.insert("v".into(), vec![first, second, third].into());
            map
        }));
    }
}
