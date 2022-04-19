use multer::Multipart;
use either::Either;

use crate::request::{Request, local_cache_once};
use crate::data::{Data, Limits, Outcome};
use crate::form::{SharedStack, prelude::*};
use crate::http::RawStr;

type Result<'r, T> = std::result::Result<T, Error<'r>>;

type Field<'r, 'i> = Either<ValueField<'r>, DataField<'r, 'i>>;

pub struct MultipartParser<'r, 'i> {
    request: &'r Request<'i>,
    buffer: &'r SharedStack<String>,
    source: Multipart<'r>,
    done: bool,
}

pub struct RawStrParser<'r> {
    buffer: &'r SharedStack<String>,
    source: &'r RawStr,
}

pub enum Parser<'r, 'i> {
    Multipart(MultipartParser<'r, 'i>),
    RawStr(RawStrParser<'r>),
}

impl<'r, 'i> Parser<'r, 'i> {
    pub async fn new(
        req: &'r Request<'i>,
        data: Data<'r>
    ) -> Outcome<'r, Parser<'r, 'i>, Errors<'r>> {
        let parser = match req.content_type() {
            Some(c) if c.is_form() => Self::from_form(req, data).await,
            Some(c) if c.is_form_data() => Self::from_multipart(req, data).await,
            _ => return Outcome::Forward(data),
        };

        match parser {
            Ok(storage) => Outcome::Success(storage),
            Err(e) => Outcome::Failure((e.status(), e.into()))
        }
    }

    async fn from_form(req: &'r Request<'i>, data: Data<'r>) -> Result<'r, Parser<'r, 'i>> {
        let limit = req.limits().get("form").unwrap_or(Limits::FORM);
        let string = data.open(limit).into_string().await?;
        if !string.is_complete() {
            Err((None, Some(limit.as_u64())))?;
        }

        Ok(Parser::RawStr(RawStrParser {
            buffer: local_cache_once!(req, SharedStack::new()),
            source: RawStr::new(local_cache_once!(req, string.into_inner())),
        }))
    }

    async fn from_multipart(req: &'r Request<'i>, data: Data<'r>) -> Result<'r, Parser<'r, 'i>> {
        let boundary = req.content_type()
            .ok_or(multer::Error::NoMultipart)?
            .param("boundary")
            .ok_or(multer::Error::NoBoundary)?;

        let form_limit = req.limits()
            .get("data-form")
            .unwrap_or(Limits::DATA_FORM);

        Ok(Parser::Multipart(MultipartParser {
            request: req,
            buffer: local_cache_once!(req, SharedStack::new()),
            source: Multipart::with_reader(data.open(form_limit), boundary),
            done: false,
        }))
    }

    pub async fn next(&mut self) -> Option<Result<'r, Field<'r, 'i>>> {
        match self {
            Parser::Multipart(ref mut p) => p.next().await,
            Parser::RawStr(ref mut p) => p.next().map(|f| Ok(Either::Left(f)))
        }
    }
}

impl<'r> RawStrParser<'r> {
    pub fn new(buffer: &'r SharedStack<String>, source: &'r RawStr) -> Self {
        RawStrParser { buffer, source }
    }
}

impl<'r> Iterator for RawStrParser<'r> {
    type Item = ValueField<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::borrow::Cow::*;

        let (name, value) = loop {
            if self.source.is_empty() {
                return None;
            }

            let (field_str, rest) = self.source.split_at_byte(b'&');
            self.source = rest;

            if !field_str.is_empty() {
                break field_str.split_at_byte(b'=');
            }
        };

        trace_!("url-encoded field: {:?}", (name, value));
        let name_val = match (name.url_decode_lossy(), value.url_decode_lossy()) {
            (Borrowed(name), Borrowed(val)) => (name, val),
            (Borrowed(name), Owned(v)) => (name, self.buffer.push(v)),
            (Owned(name), Borrowed(val)) => (self.buffer.push(name), val),
            (Owned(mut name), Owned(val)) => {
                let len = name.len();
                name.push_str(&val);
                self.buffer.push_split(name, len)
            }
        };

        Some(ValueField::from(name_val))
    }
}

#[cfg(test)]
mod raw_str_parse_tests {
    use crate::form::ValueField as Field;

    #[test]
    fn test_skips_empty() {
        let buffer = super::SharedStack::new();
        let fields: Vec<_> = super::RawStrParser::new(&buffer, "a&b=c&&&c".into()).collect();
        assert_eq!(fields, &[Field::parse("a"), Field::parse("b=c"), Field::parse("c")]);
    }

    #[test]
    fn test_decodes() {
        let buffer = super::SharedStack::new();
        let fields: Vec<_> = super::RawStrParser::new(&buffer, "a+b=c%20d&%26".into()).collect();
        assert_eq!(fields, &[Field::parse("a b=c d"), Field::parse("&")]);
    }
}

impl<'r, 'i> MultipartParser<'r, 'i> {
    /// Returns `None` when there are no further fields. Otherwise tries to
    /// parse the next multipart form field and returns the result.
    async fn next(&mut self) -> Option<Result<'r, Field<'r, 'i>>> {
        if self.done {
            return None;
        }

        let field = match self.source.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => return None,
            Err(e) => {
                self.done = true;
                return Some(Err(e.into()));
            }
        };

        // A field with a content-type is data; one without is "value".
        trace_!("multipart field: {:?}", field);
        let content_type = field.content_type().and_then(|m| m.as_ref().parse().ok());
        let field = if let Some(content_type) = content_type {
            let (name, file_name) = match (field.name(), field.file_name()) {
                (None, None) => ("", None),
                (None, Some(file_name)) => ("", Some(self.buffer.push(file_name))),
                (Some(name), None) => (self.buffer.push(name), None),
                (Some(a), Some(b)) => {
                    let (field_name, file_name) = self.buffer.push_two(a, b);
                    (field_name, Some(file_name))
                }
            };

            Either::Right(DataField {
                content_type,
                request: self.request,
                name: NameView::new(name),
                file_name: file_name.map(crate::fs::FileName::new),
                data: Data::from(field),
            })
        } else {
            let (mut buf, len) = match field.name() {
                Some(s) => (s.to_string(), s.len()),
                None => (String::new(), 0)
            };

            match field.text().await {
                Ok(text) => buf.push_str(&text),
                Err(e) => return Some(Err(e.into())),
            };

            let name_val = self.buffer.push_split(buf, len);
            Either::Left(ValueField::from(name_val))
        };

        Some(Ok(field))
    }
}
