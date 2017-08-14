use std::fmt;
use std::collections::BTreeMap;

use config::Value;

pub fn parse_simple_toml_value(string: &str) -> Result<Value, &'static str>  {
    if string.is_empty() {
        return Err("value is empty")
    }

    let value = if let Ok(int) = string.parse::<i64>() {
        Value::Integer(int)
    } else if let Ok(float) = string.parse::<f64>() {
        Value::Float(float)
    } else if let Ok(boolean) = string.parse::<bool>() {
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
        assert_parse!("[1]", vec![1].into());
        assert_parse!("[1, 2, 3]", vec![1, 2, 3].into());
        assert_parse!("[1.32, 2]", Array(vec![1.32.into(), 2.into()]));

        assert_parse!("{}", Table(BTreeMap::new()));
        assert_parse!("{a=b}", Table({
            let mut map = BTreeMap::new();
            map.insert("a".into(), "b".into());
            map
        }));
        assert_parse!("{v=1, on=true,pi=3.14}", Table({
            let mut map = BTreeMap::new();
            map.insert("v".into(), 1.into());
            map.insert("on".into(), true.into());
            map.insert("pi".into(), 3.14.into());
            map
        }));
    }
}
