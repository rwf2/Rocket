#[derive(Debug, Clone)]
pub struct Segment {
    /// The name of the parameter or just the static string.
    pub value: String,
    /// This is a `<a>`.
    pub dynamic: bool,
    /// This is a `<a..>`.
    pub dynamic_trail: bool,
}

impl Segment {
    pub fn from(segment: &crate::http::RawStr) -> Self {
        let mut value = segment;
        let mut dynamic = false;
        let mut dynamic_trail = false;

        if segment.starts_with('<') && segment.ends_with('>') {
            dynamic = true;
            value = &segment[1..(segment.len() - 1)];

            if value.ends_with("..") {
                dynamic_trail = true;
                value = &value[..(value.len() - 2)];
            }
        }

        Segment { value: value.to_string(), dynamic, dynamic_trail }
    }
}
