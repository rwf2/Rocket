use toml;

use toml::Value;
use serde::de::Deserialize;

use config::ConfigError;
use super::Result;

pub fn parse(toml: &str) -> Result<Value> {
    let first_error = match toml.parse() {
        Ok(ret) => return Ok(ret),
        Err(e) => e,
    };

    let mut second_parser = toml::de::Deserializer::new(toml);

    if let Ok(ret) = Value::deserialize(&mut second_parser) {
        return Ok(ret)
    }

    Err(ConfigError::from(first_error))
}
