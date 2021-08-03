use super::{header_utilities, header_names, RangeItemHeaderValue, Header};


pub struct ContentRangeHeaderValue {
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub length: Option<u64>,
    pub unit: String,
}

impl ContentRangeHeaderValue {
    pub fn has_length(&self) -> bool {
        self.length.is_some()
    }

    pub fn has_range(&self) -> bool {
        self.from.is_some()
    }
}

impl From<u64> for ContentRangeHeaderValue {
    fn from(len: u64) -> Self {
        Self {
            from: None,
            to: None,
            length: Some(len),
            unit: header_utilities::BYTES_UNIT.to_string()
        }
    }
}

impl From<(u64, u64, u64)> for ContentRangeHeaderValue {
    fn from((from, to, len): (u64, u64, u64)) -> Self {
        Self {
            from: Some(from),
            to: Some(to),
            length: Some(len),
            unit: header_utilities::BYTES_UNIT.to_string()
        }
    }
}
/// Scenario: "Content-Range: bytes 12-34/*"
impl From<(u64, u64)> for ContentRangeHeaderValue {
    fn from((from, to): (u64, u64)) -> Self {
        Self {
            from: Some(from),
            to: Some(to),
            length: None,
            unit: header_utilities::BYTES_UNIT.to_string()
        }
    }
}

impl ToString for ContentRangeHeaderValue {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(self.unit.as_str());
        s.push(' ');

        if self.has_range(){
            s.push_str(self.from.unwrap().to_string().as_str());
            s.push('-');
            if let Some(to) = self.to {
                s.push_str(to.to_string().as_str());
            }
        } else {
            s.push('*');
        }

        s.push('/');
        if self.has_length() {
            s.push_str(self.length.unwrap().to_string().as_str());
        } else {
            s.push('*');
        }
        s
    }
}

impl<'h> Into<Header<'h>> for ContentRangeHeaderValue {
    fn into(self) -> Header<'h> {
        Header {
            name: header_names::CONTENT_RANGE.into(),
            value: self.to_string().into()
        }
    }
}
