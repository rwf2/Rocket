use crate::http::RawStr;

#[derive(Debug, Clone)]
pub struct Segment {
    pub value: String,
    pub dynamic: bool,
    pub trailing: bool,
}

impl Segment {
    pub fn from(segment: &RawStr) -> Self {
        let mut value = segment;
        let mut dynamic = false;
        let mut trailing = false;
        let mut n = segment.len();

        if segment.starts_with('<') && segment.ends_with('>') {
            dynamic = true;
            n -= 1;
        }
        else if segment.starts_with('{') && segment.ends_with('}') {
            dynamic = true;
            n -= 1;
        } else if segment.starts_with(':') {
            dynamic = true;
        }

        if dynamic {
            value = &segment[1..n];

            if value.ends_with("..") {
                trailing = true;
                value = &value[..(value.len() - 2)];
            }
        }

        Segment { value: value.to_string(), dynamic, trailing }
    }
}
