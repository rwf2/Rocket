use std::collections::{HashMap, BTreeMap};
use std::hash::Hash;
use std::str::FromStr;

use config::Value;

pub fn parse_simple_toml_value(string: &str) -> Value {
    if let Ok(int) = i64::from_str(string) {
        return Value::Integer(int)
    }

    if let Ok(boolean) = bool::from_str(string) {
        return Value::Boolean(boolean)
    }

    if let Ok(float) = f64::from_str(string) {
        return Value::Float(float)
    }

    Value::String(string.to_string())
}

/// Conversion trait from standard types into TOML `Value`s.
pub trait IntoValue {
    /// Converts `self` into a TOML `Value`.
    fn into_value(self) -> Value;
}

impl<'a> IntoValue for &'a str {
    fn into_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl<V: IntoValue> IntoValue for Vec<V> {
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

