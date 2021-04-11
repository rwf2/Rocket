use crate::{
    form::{name::NameBuf, ValueField},
    http::uri::Uri,
};
use rocket_http::{ContentType, RawStr};

/// A struct that can be used to easily create simple and multipart form data
/// for testing purposes.
///
/// ## Example
///
/// The following snippet creates a request with a form submission:
///
/// ```rust,no_run
/// use rocket::local::form::LocalForm;
/// use rocket::local::blocking::{Client, LocalRequest};
/// use rocket::http::{ContentType};
///
/// let client = Client::tracked(rocket::ignite()).expect("valid rocket");
/// let req = client.post("/")
///     .form(LocalForm::new()
///             .field("field", "value")
///             .file("foo.txt", ContentType::Plain, "hi there!"));
///
/// let response = req.dispatch();
/// ```
pub struct LocalForm<'v>(Vec<LocalField<'v>>);

#[derive(Debug, PartialEq)]
pub(crate) enum LocalField<'v> {
    Value(NameBuf<'v>, String),
    Data(
        NameBuf<'v>,
        Option<&'v str>,
        ContentType,
        Vec<u8>,
    ),
}

impl<'v> LocalForm<'v> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// A percent-decoded `name` and `value`.
    pub fn field<N, V>(mut self, name: N, value: V) -> Self
    where
        N: Into<NameBuf<'v>>,
        V: AsRef<str>,
    {
        self.0
            .push(LocalField::Value(name.into(), value.as_ref().to_owned()));
        self
    }

    /// Adds all the `fields` to `self`.
    pub fn fields<I, F>(mut self, fields: I) -> Self
    where
        I: Iterator<Item = F>,
        F: Into<ValueField<'v>>,
    {
        fields.for_each(|field| {
            let field = field.into();
            self.0.push(LocalField::Value(
                NameBuf::from(field.name.as_name()),
                field.value.to_string(),
            ));
        });
        self
    }

    /// A percent-encoded `name` and `value`.
    pub fn raw_field(mut self, name: &'v RawStr, value: &'v RawStr) -> Self {
        let decoded_name = name.percent_decode_lossy().into_owned();
        let value = value.percent_decode_lossy().into_owned();
        self.0
            .push(LocalField::Value(NameBuf::from(decoded_name), value));
        self
    }

    /// Adds a field for the file with the name `file_name`, content-type `ct`, and
    /// file contents `data`.
    pub fn file<N, V>(mut self, file_name: N, ct: ContentType, data: V) -> Self
    where
        N: Into<Option<&'v str>>,
        V: AsRef<[u8]>,
    {
        self.0.push(LocalField::Data(
            NameBuf::from("file"),
            file_name.into(),
            ct,
            data.as_ref().into(),
        ));
        self
    }

    /// Add a data field with a content-type `ct` and binary `data`.
    pub fn data<V>(mut self, ct: ContentType, data: V) -> Self
    where
        V: AsRef<[u8]>,
    {
        self.0.push(LocalField::Data(
            NameBuf::from("file"),
            None,
            ct,
            data.as_ref().into(),
        ));
        self
    }

    /// The full content-type for this form.
    pub fn content_type(&self) -> ContentType {
        if self.contains_data_field() {
            return "multipart/form-data; boundary=X-BOUNDARY"
                .parse::<ContentType>()
                .unwrap();
        }

        ContentType::Form
    }

    /// The full body data for this form.
    pub fn body_data(&self) -> Vec<u8> {
        if self.contains_data_field() {
            self.format_multipart()
        } else {
            self.format_simple()
        }
    }

    fn contains_data_field(&self) -> bool {
        self
            .0
            .iter()
            .any(|field| matches!(field, LocalField::Data(..)))
    }

    fn format_simple(&self) -> Vec<u8> {
        self.0.iter().fold(Vec::new(), |mut acc, field| {
            match field {
                LocalField::Value(name, value) => {
                    acc.push(format!("{}={}", Uri::percent_encode(&format!("{}", name)), Uri::percent_encode(value)));
                    return acc
                },
                _ => acc,
            }
        }).join("&").as_bytes().to_vec()
    }

    fn format_multipart(&self) -> Vec<u8> {
        let mut body = self.0.iter().fold(Vec::new(), |mut acc, field| {
            match field {
                LocalField::Value(name, value) => {
                    acc.push("--X-BOUNDARY".to_string());
                    acc.push(format!("Content-Disposition: form-data; name=\"{}\"", name));
                    acc.push("".to_string());
                    acc.push(format!("{}", value));
                },
                LocalField::Data(name, file_name, content_type, data) => {
                    acc.push("--X-BOUNDARY".to_string());
                    acc.push(format!("Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"", name, file_name.unwrap_or("")));
                    acc.push(format!("Content-Type: {}", content_type));
                    acc.push("".to_string());
                    acc.push(format!("{}", String::from_utf8_lossy(data)));
                },
            }
            return acc
        });
        body.push("--X-BOUNDARY--".to_string());
        body.push("".to_string());
        body.join("\r\n").as_bytes().to_vec()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_content_type() {
        let form = LocalForm::new()
            .field("name[]", "john doe");
        assert_eq!(ContentType::Form, form.content_type());

        let form = form.file("foo.txt", ContentType::Plain, "123");
        assert_eq!(ContentType::FormData, form.content_type());
    }

    #[test]
    fn test_body_data() {
        let simple_body = "field=value&is%20it=a%20cat%3F";
        let form = LocalForm::new()
            .field("field", "value")
            .field("is it", "a cat?");

        assert_eq!(simple_body, String::from_utf8_lossy(&form.body_data()));

        let multipart_body = &[
            "--X-BOUNDARY",
            r#"Content-Disposition: form-data; name="names[]""#,
            "",
            "abcd",
            "--X-BOUNDARY",
            r#"Content-Disposition: form-data; name="names[]""#,
            "",
            "123",
            "--X-BOUNDARY",
            r#"Content-Disposition: form-data; name="file"; filename="foo.txt""#,
            "Content-Type: text/plain; charset=utf-8",
            "",
            "hi there",
            "--X-BOUNDARY--",
            "",
        ].join("\r\n");
        let form = LocalForm::new()
            .field("names[]", "abcd")
            .field("names[]", "123")
            .file("foo.txt", ContentType::Plain, "hi there");

        assert_eq!(multipart_body.as_str(), String::from_utf8_lossy(&form.body_data()));
    }
}
