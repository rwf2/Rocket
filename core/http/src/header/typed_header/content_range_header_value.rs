use super::{header_utilities, header_names, Header};

/// Represents a `Content-Range` response HTTP header.
#[derive(Clone)]
pub struct ContentRangeHeaderValue {
    /// The start of the range.
    pub from: Option<u64>,

    /// The end of the range.
    pub to: Option<u64>,

    /// The total size of the document.
    pub length: Option<u64>,

    /// The unit in which ranges are specified.
    pub unit: String,
}

impl ContentRangeHeaderValue {
    ///  Gets a value that determines if `length` has been specified.
    pub fn has_length(&self) -> bool {
        self.length.is_some()
    }

    /// Gets a value that determines if `from` and `to` have been specified.
    pub fn has_range(&self) -> bool {
        self.from.is_some() && self.to.is_some()
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

impl From<ContentRangeHeaderValue> for Header<'_> {
    fn from(v: ContentRangeHeaderValue) -> Self {
        Header {
            name: header_names::CONTENT_RANGE.into(),
            value: v.to_string().into()
        }
    }
}
