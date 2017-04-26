use std::fmt;
use std::collections::{HashMap, BTreeMap};
use std::hash::Hash;
use std::str::FromStr;

use config::Value;

pub fn parse_simple_toml_value(string: &str) -> Result<Value, &'static str>  {
    if string.is_empty() {
        return Err("value is empty")
    }

    let value = if let Ok(int) = i64::from_str(string) {
        Value::Integer(int)
    } else if let Ok(float) = f64::from_str(string) {
        Value::Float(float)
    } else if let Ok(boolean) = bool::from_str(string) {
        Value::Boolean(boolean)
    } else if string.starts_with('{') {
        if !string.ends_with('}') {
            return Err("value is missing closing '}'")
        }

        let mut table = BTreeMap::new();
        let inner = &string[1..string.len() - 1].trim();
        if !inner.is_empty() {
            for key_val in inner.split(',') {
                let (key, val) = match key_val.find('=') {
                    Some(i) => (&key_val[..i], &key_val[(i + 1)..]),
                    None => return Err("missing '=' in dicitonary key/value pair")
                };

                let key = key.trim().to_string();
                let val = parse_simple_toml_value(val.trim())?;
                table.insert(key, val);
            }
        }

        Value::Table(table)
    } else if string.starts_with('[') {
        if !string.ends_with(']') {
            return Err("value is missing closing ']'")
        }

        let mut vals = vec![];
        let inner = &string[1..string.len() - 1].trim();
        if !inner.is_empty() {
            for val_str in inner.split(',') {
                vals.push(parse_simple_toml_value(val_str.trim())?);
            }
        }

        Value::Array(vals)
    } else if string.starts_with('"') {
        if !string[1..].ends_with('"') {
            return Err("value is missing closing '\"'");
        }

        Value::String(string[1..string.len() - 1].to_string())
    } else {
        Value::String(string.to_string())
    };

    Ok(value)
}

/// Conversion trait from standard types into TOML `Value`s.
pub trait IntoValue {
    /// Converts `self` into a TOML `Value`.
    fn into_value(self) -> Value;
}

impl<'a> IntoValue for &'a str {
    #[inline(always)]
    fn into_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoValue for Value {
    #[inline(always)]
    fn into_value(self) -> Value {
        self
    }
}

impl<V: IntoValue> IntoValue for Vec<V> {
    #[inline(always)]
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(|v| v.into_value()).collect())
    }
}

impl<S: Into<String>, V: IntoValue> IntoValue for BTreeMap<S, V> {
    fn into_value(self) -> Value {
        let table = self.into_iter()
            .map(|(s, v)| (s.into(), v.into_value()))
            .collect();

        Value::Table(table)
    }
}

impl<S: Into<String> + Hash + Eq, V: IntoValue> IntoValue for HashMap<S, V> {
    fn into_value(self) -> Value {
        let table = self.into_iter()
            .map(|(s, v)| (s.into(), v.into_value()))
            .collect();

        Value::Table(table)
    }
}

macro_rules! impl_into_value {
    ($variant:ident : $t:ty) => ( impl_into_value!($variant: $t,); );

    ($variant:ident : $t:ty, $($extra:tt)*) => (
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::$variant(self $($extra)*)
            }
        }
    )
}

impl_into_value!(String: String);
impl_into_value!(Integer: i64);
impl_into_value!(Integer: isize, as i64);
impl_into_value!(Integer: i32, as i64);
impl_into_value!(Integer: i8, as i64);
impl_into_value!(Integer: u8, as i64);
impl_into_value!(Integer: u32, as i64);
impl_into_value!(Boolean: bool);
impl_into_value!(Float: f64);
impl_into_value!(Float: f32, as f64);

/// A simple wrapper over a `Value` reference with a custom implementation of
/// `Display`. This is used to log config values at initialization.
pub(crate) struct LoggedValue<'a>(pub &'a Value);

impl<'a> fmt::Display for LoggedValue<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use config::Value::*;
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
    use std::collections::BTreeMap;
    use super::parse_simple_toml_value;
    use super::IntoValue;
    use super::Value::*;

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
        assert_parse!("hello, WORLD!", String("hello, WORLD!".into()));
        assert_parse!("hi", String("hi".into()));
        assert_parse!("\"hi\"", String("hi".into()));

        assert_parse!("[]", Array(Vec::new()));
        assert_parse!("[1]", vec![1].into_value());
        assert_parse!("[1, 2, 3]", vec![1, 2, 3].into_value());
        assert_parse!("[1.32, 2]",
                      vec![1.32.into_value(), 2.into_value()].into_value());

        assert_parse!("{}", Table(BTreeMap::new()));
        assert_parse!("{a=b}", Table({
            let mut map = BTreeMap::new();
            map.insert("a".into(), "b".into_value());
            map
        }));
        assert_parse!("{v=1, on=true,pi=3.14}", Table({
            let mut map = BTreeMap::new();
            map.insert("v".into(), 1.into_value());
            map.insert("on".into(), true.into_value());
            map.insert("pi".into(), 3.14.into_value());
            map
        }));
    }
}
