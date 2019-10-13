#[derive(Debug, Clone)]
pub struct CorsFairingConfig {
    pub headers: Vec<String>,
    pub origin: String
}

impl CorsFairingConfig {
    pub fn new(headers: Vec<String>, origin: String) -> CorsFairingConfig {
        CorsFairingConfig {
            headers: headers,
            origin: origin
        }
    }

    pub fn with_any_origin() -> CorsFairingConfig {
        CorsFairingConfig {
            headers: Vec::new(),
            origin: "*".to_owned()
        }
    }

    pub fn with_origin(origin: String) -> CorsFairingConfig {
        CorsFairingConfig {
            headers: Vec::new(),
            origin: origin
        }
    }
}
