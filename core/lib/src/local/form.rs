use crate::form::{name::NameBuf, ValueField};
use rocket_http::{ContentType, RawStr};

pub struct LocalForm(Vec<LocalField>);

pub enum LocalField {
    Value(NameBuf<'static>, String),
    Data(
        NameBuf<'static>,
        Option<&'static str>,
        Option<ContentType>,
        Vec<u8>,
    ),
}

impl LocalForm {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// A percent-decoded `name` and `value`.
    pub fn field<N, V>(mut self, name: N, value: V) -> Self
    where
        N: Into<NameBuf<'static>>,
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
        F: Into<ValueField<'static>>,
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
    pub fn raw_field(mut self, name: &'static RawStr, value: &'static RawStr) -> Self {
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
        N: Into<Option<&'static str>>,
        V: AsRef<[u8]>,
    {
        self.0.push(LocalField::Data(
            NameBuf::from("file"),
            file_name.into(),
            Some(ct),
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
            Some(ct),
            data.as_ref().into(),
        ));
        self
    }

    /// The full content-type for this form.
    pub fn content_type(&self) -> ContentType {
        if self
            .0
            .iter()
            .any(|field| matches!(field, LocalField::Data(..)))
        {
            return ContentType::FormData;
        }

        ContentType::Form
    }

    /// The full body data for this form.
    pub fn body_data(&self) -> Vec<u8> {
        todo!()
    }
}

impl<F: Into<ValueField<'static>>, I: Iterator<Item = F>> From<I> for LocalForm {
    fn from(fields: I) -> Self {
        LocalForm::new().fields(fields)
    }
}
