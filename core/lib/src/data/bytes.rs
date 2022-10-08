use std::borrow::Cow;
use std::io;

use crate::data::{Capped, FromData, Limits, Outcome, N};
use crate::form::{DataField, Errors, FromFormField, ValueField};
use crate::http::{ContentType, Status};
use crate::{fs::FileName, outcome::IntoOutcome, Data, Request};

#[derive(Debug)]
pub struct Bytes<'v> {
    pub file_name: Option<&'v FileName>,
    pub content_type: Option<ContentType>,
    pub content: Cow<'v, [u8]>,
}

impl<'v> Bytes<'v> {
    async fn from<'a>(
        req: &Request<'_>,
        data: Data<'_>,
        file_name: Option<&'a FileName>,
        content_type: Option<ContentType>,
    ) -> io::Result<Capped<Bytes<'a>>> {
        let limit = { content_type.as_ref() }
            .and_then(|ct| ct.extension()?.limits().find(&["file", ext.as_str()]))
            .or_else(|| req.limits().get("file"))
            .unwrap_or(Limits::FILE);

        let Capped { value, n } = data.open(limit).into_bytes().await?;

        Ok(Capped::new(
            Bytes {
                content_type,
                file_name,
                content: value.into(),
            },
            n,
        ))
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<Bytes<'v>> {
    fn from_value(field: ValueField<'v>) -> Result<Self, Errors<'v>> {
        let n = N {
            written: field.value.len() as u64,
            complete: true,
        };
        Ok(Capped::new(
            Bytes {
                file_name: None,
                content_type: None,
                content: field.value.as_bytes().into(),
            },
            n,
        ))
    }

    async fn from_data(f: DataField<'v, '_>) -> Result<Self, Errors<'v>> {
        Ok(Bytes::from(f.request, f.data, f.file_name, Some(f.content_type)).await?)
    }
}

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<Bytes<'_>> {
    type Error = io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        use yansi::Paint;

        let has_form = |ty: &ContentType| ty.is_form_data() || ty.is_form();
        if req.content_type().map_or(false, has_form) {
            let (tf, form) = (Paint::white("Bytes<'_>"), Paint::white("Form<Bytes<'_>>"));
            warn_!("Request contains a form that will not be processed.");
            info_!(
                "Bare `{}` data guard writes raw, unprocessed streams to disk.",
                tf
            );
            info_!("Did you mean to use `{}` instead?", form);
        }

        Bytes::from(req, data, None, req.content_type().cloned())
            .await
            .into_outcome(Status::BadRequest)
    }
}

impl_strict_from_form_field_from_capped!(Bytes<'v>);
impl_strict_from_data_from_capped!(Bytes<'_>);
