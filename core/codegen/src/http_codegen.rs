use quote::ToTokens;
use devise::{FromMeta, MetaItem, Result, ext::{Split2, PathExt, SpanDiagnosticExt}};

use crate::proc_macro2::TokenStream;
use crate::http;

#[derive(Debug)]
pub struct ContentType(pub http::ContentType);

#[derive(Debug)]
pub struct Status(pub http::Status);

#[derive(Debug)]
pub struct MediaType(pub http::MediaType);

#[derive(Debug, Copy, Clone)]
pub struct Method(pub http::Method);

#[derive(Clone, Debug)]
pub struct Optional<T>(pub Option<T>);

impl FromMeta for Status {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        let num = usize::from_meta(meta)?;
        if num < 100 || num >= 600 {
            return Err(meta.value_span().error("status must be in range [100, 599]"));
        }

        Ok(Status(http::Status::new(num as u16)))
    }
}

impl ToTokens for Status {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let code = self.0.code;
        tokens.extend(quote!(rocket::http::Status { code: #code }));
    }
}

impl FromMeta for ContentType {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        http::ContentType::parse_flexible(&String::from_meta(meta)?)
            .map(ContentType)
            .ok_or(meta.value_span().error("invalid or unknown content type"))
    }
}

impl ToTokens for ContentType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let http_media_type = self.0.media_type().clone();
        let media_type = MediaType(http_media_type);
        tokens.extend(quote!(::rocket::http::ContentType(#media_type)));
    }
}

impl FromMeta for MediaType {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        let mt = http::MediaType::parse_flexible(&String::from_meta(meta)?)
            .ok_or(meta.value_span().error("invalid or unknown media type"))?;

        if !mt.is_known() {
            // FIXME(diag: warning)
            meta.value_span()
                .warning(format!("'{}' is not a known media type", mt))
                .emit_as_item_tokens();
        }

        Ok(MediaType(mt))
    }
}

impl ToTokens for MediaType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (top, sub) = (self.0.top().as_str(), self.0.sub().as_str());
        let (keys, values) = self.0.params().map(|(k, v)| (k.as_str(), v)).split2();
        let http = quote!(::rocket::http);

        tokens.extend(quote! {
            #http::MediaType::const_new(#top, #sub, &[#((#keys, #values)),*])
        });
    }
}

const VALID_METHODS_STR: &str = "`GET`, `PUT`, `POST`, `DELETE`, `HEAD`, \
    `PATCH`, `OPTIONS`";

const VALID_METHODS: &[http::Method] = &[
    http::Method::Get, http::Method::Put, http::Method::Post,
    http::Method::Delete, http::Method::Head, http::Method::Patch,
    http::Method::Options,
];

impl FromMeta for Method {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        let span = meta.value_span();
        let help_text = format!("method must be one of: {}", VALID_METHODS_STR);

        if let MetaItem::Path(path) = meta {
            if let Some(ident) = path.last_ident() {
                let method = ident.to_string().parse()
                    .map_err(|_| span.error("invalid HTTP method").help(&*help_text))?;

                if !VALID_METHODS.contains(&method) {
                    return Err(span.error("invalid HTTP method for route handlers")
                               .help(&*help_text));
                }

                return Ok(Method(method));
            }
        }

        Err(span.error(format!("expected identifier, found {}", meta.description()))
                .help(&*help_text))
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let method_tokens = match self.0 {
            http::Method::Get => quote!(::rocket::http::Method::Get),
            http::Method::Put => quote!(::rocket::http::Method::Put),
            http::Method::Post => quote!(::rocket::http::Method::Post),
            http::Method::Delete => quote!(::rocket::http::Method::Delete),
            http::Method::Options => quote!(::rocket::http::Method::Options),
            http::Method::Head => quote!(::rocket::http::Method::Head),
            http::Method::Trace => quote!(::rocket::http::Method::Trace),
            http::Method::Connect => quote!(::rocket::http::Method::Connect),
            http::Method::Patch => quote!(::rocket::http::Method::Patch),
        };

        tokens.extend(method_tokens);
    }
}

impl<T: ToTokens> ToTokens for Optional<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use crate::exports::{_Some, _None};
        use devise::Spanned;

        let opt_tokens = match self.0 {
            Some(ref val) => quote_spanned!(val.span() => #_Some(#val)),
            None => quote!(#_None)
        };

        tokens.extend(opt_tokens);
    }
}
