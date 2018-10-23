use quote::ToTokens;
use proc_macro2::TokenStream as TokenStream2;
use derive_utils::{FromMeta, MetaItem, Result, ext::Split2};
use http::{self, ext::IntoOwned};
use attribute::segments::{parse_segments, parse_segment, Segment, Kind, Source};

use proc_macro_ext::SpanExt;

#[derive(Debug)]
crate struct ContentType(crate http::ContentType);

#[derive(Debug)]
crate struct Status(crate http::Status);

#[derive(Debug)]
crate struct MediaType(crate http::MediaType);

#[derive(Debug)]
crate struct Method(crate http::Method);

#[derive(Debug)]
crate struct Origin(crate http::uri::Origin<'static>);

#[derive(Clone, Debug)]
crate struct DataSegment(crate Segment);

#[derive(Clone, Debug)]
crate struct Optional<T>(crate Option<T>);

#[derive(Debug)]
crate struct RoutePath {
    crate origin: Origin,
    crate path: Vec<Segment>,
    crate query: Option<Vec<Segment>>,
}

impl FromMeta for Status {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let num = usize::from_meta(meta)?;
        if num < 100 || num >= 600 {
            return Err(meta.value_span().error("status must be in range [100, 599]"));
        }

        Ok(Status(http::Status::raw(num as u16)))
    }
}

impl ToTokens for Status {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let (code, reason) = (self.0.code, self.0.reason);
        tokens.extend(quote!(rocket::http::Status { code: #code, reason: #reason }));
    }
}

impl FromMeta for ContentType {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        http::ContentType::parse_flexible(&String::from_meta(meta)?)
            .map(ContentType)
            .ok_or(meta.value_span().error("invalid or unknown content type"))
    }
}

impl ToTokens for ContentType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // Yeah, yeah. (((((i))).kn0w()))
        let media_type = MediaType((self.0).clone().0);
        tokens.extend(quote!(::rocket::http::ContentType(#media_type)));
    }
}

impl FromMeta for MediaType {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let mt = http::MediaType::parse_flexible(&String::from_meta(meta)?)
            .ok_or(meta.value_span().error("invalid or unknown media type"))?;

        if !mt.is_known() {
            meta.value_span()
                .warning(format!("'{}' is not a known media type", mt))
                .emit();
        }

        Ok(MediaType(mt))
    }
}

impl ToTokens for MediaType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        use std::iter::repeat;
        let (top, sub) = (self.0.top().as_str(), self.0.sub().as_str());
        let (keys, values) = self.0.params().split2();

        let (http, cow) = (quote!(::rocket::http), quote!(::std::borrow::Cow));
        let (http_, http__) = (repeat(&http), repeat(&http));
        let (cow_, cow__) = (repeat(&cow), repeat(&cow));

        // TODO: Produce less code when possible (for known media types).
        tokens.extend(quote!(#http::MediaType {
            source: #http::Source::None,
            top: #http::Indexed::Concrete(#cow::Borrowed(#top)),
            sub: #http::Indexed::Concrete(#cow::Borrowed(#sub)),
            params: #http::MediaParams::Static(&[
                #((
                    #http_::Indexed::Concrete(#cow_::Borrowed(#keys)),
                    #http__::Indexed::Concrete(#cow__::Borrowed(#values))
                )),*
            ])
        }))
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
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let span = meta.value_span();
        let help_text = format!("method must be one of: {}", VALID_METHODS_STR);

        if let MetaItem::Ident(ident) = meta {
            let method = ident.to_string().parse()
                .map_err(|_| span.error("invalid HTTP method").help(&*help_text))?;

            if !VALID_METHODS.contains(&method) {
                return Err(span.error("invalid HTTP method for route handlers")
                               .help(&*help_text));
            }

            return Ok(Method(method));
        }

        Err(span.error(format!("expected identifier, found {}", meta.description()))
                .help(&*help_text))
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
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

impl FromMeta for Origin {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let string = String::from_meta(meta)?;
        let span = meta.value_span();

        let uri = http::uri::Origin::parse_route(&string)
            .map_err(|e| {
                let span = e.index()
                    .map(|i| span.subspan(i + 1..).expect("origin"))
                    .unwrap_or(span);

                span.error(format!("invalid path URI: {}", e))
                    .help("expected path in origin form: \"/path/<param>\"")
            })?;

        if !uri.is_normalized() {
            let normalized = uri.to_normalized();
            return Err(span.error("paths cannot contain empty segments")
                .note(format!("expected '{}', found '{}'", normalized, uri)));
        }

        Ok(Origin(uri.into_owned()))
    }
}

impl FromMeta for Segment {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let string = String::from_meta(meta)?;
        let span = meta.value_span()
            .subspan(1..(string.len() + 1))
            .expect("segment");

        let segment = parse_segment(&string, span)?;
        if segment.kind != Kind::Single {
            return Err(span.error("malformed parameter")
                        .help("parameter must be of the form '<param>'"));
        }

        Ok(segment)
    }
}

impl FromMeta for DataSegment {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let mut segment = Segment::from_meta(meta)?;
        segment.source = Source::Data;
        segment.index = Some(0);
        Ok(DataSegment(segment))
    }
}

impl FromMeta for RoutePath {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let (origin, span) = (Origin::from_meta(meta)?, meta.value_span());
        let path_span = span.subspan(1..origin.0.path().len() + 1).expect("path");
        let path = parse_segments(origin.0.path(), '/', Source::Path, path_span);

        let query = origin.0.query()
            .map(|q| {
                let len_to_q = 1 + origin.0.path().len() + 1;
                let end_of_q = len_to_q + q.len();
                let query_span = span.subspan(len_to_q..end_of_q).expect("query");
                if q.starts_with('&') || q.contains("&&") || q.ends_with('&') {
                    // TODO: Show a help message with what's expected.
                    Err(query_span.error("query cannot contain empty segments").into())
                } else {
                    parse_segments(q, '&', Source::Query, query_span)
                }
            }).transpose();

        match (path, query) {
            (Ok(path), Ok(query)) => Ok(RoutePath { origin, path, query }),
            (Err(diag), Ok(_)) | (Ok(_), Err(diag)) => Err(diag.emit_head()),
            (Err(d1), Err(d2)) => Err(d1.join(d2).emit_head())
        }
    }
}

impl<T: ToTokens> ToTokens for Optional<T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let opt_tokens = match self.0 {
            Some(ref val) => quote!(Some(#val)),
            None => quote!(None)
        };

        tokens.extend(opt_tokens);
    }
}
