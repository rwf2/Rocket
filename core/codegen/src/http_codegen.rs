use devise::{
    ext::{SpanDiagnosticExt, Split2},
    FromMeta, MetaItem, Result,
};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;

use crate::{attribute::suppress::Lint, http};

#[derive(Debug)]
pub struct ContentType(pub http::ContentType);

#[derive(Debug)]
pub struct Status(pub http::Status);

#[derive(Debug)]
pub struct MediaType(pub http::MediaType);

#[derive(Debug, Clone)]
pub struct Method(pub http::Method);

#[derive(Clone, Debug)]
pub struct Optional<T>(pub Option<T>);

#[derive(Debug)]
pub struct Origin<'a>(pub &'a http::uri::Origin<'a>, pub Span);

#[derive(Debug)]
pub struct Absolute<'a>(pub &'a http::uri::Absolute<'a>, pub Span);

#[derive(Debug)]
pub struct Authority<'a>(pub &'a http::uri::Authority<'a>, pub Span);

#[derive(Debug)]
pub struct Reference<'a>(pub &'a http::uri::Reference<'a>, pub Span);

#[derive(Debug)]
pub struct Asterisk(pub http::uri::Asterisk, pub Span);

impl FromMeta for Status {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        let num = usize::from_meta(meta)?;
        if num < 100 || num >= 600 {
            return Err(meta
                .value_span()
                .error("status must be in range [100, 599]"));
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
            .ok_or_else(|| meta.value_span().error("invalid or unknown content type"))
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
            .ok_or_else(|| meta.value_span().error("invalid or unknown media type"))?;

        let lint = Lint::UnknownFormat;
        if !mt.is_known() && lint.enabled(meta.value_span()) {
            meta.value_span()
                .warning(format!("'{}' is not a known format or media type", mt))
                .note(lint.how_to_suppress())
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

impl FromMeta for Method {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        let span = meta.value_span();
        let help = format!("known methods: {}", http::Method::ALL.join(", "));

        let string = meta
            .path()
            .ok()
            .and_then(|p| p.get_ident().cloned())
            .map(|ident| (ident.span(), ident.to_string()))
            .or_else(|| match meta.lit() {
                Ok(syn::Lit::Str(s)) => Some((s.span(), s.value())),
                _ => None,
            });

        if let Some((span, string)) = string {
            string
                .to_ascii_uppercase()
                .parse()
                .map(Method)
                .map_err(|_| span.error("invalid or unknown HTTP method").help(help))
        } else {
            let err = format!(
                "expected method ident or string, found {}",
                meta.description()
            );
            Err(span.error(err).help(help))
        }
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variant = syn::Ident::new(self.0.variant_str(), Span::call_site());
        tokens.extend(quote!(::rocket::http::Method::#variant));
    }
}

impl<T: ToTokens> ToTokens for Optional<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use crate::exports::{_None, _Some};
        use devise::Spanned;

        let opt_tokens = match self.0 {
            Some(ref val) => quote_spanned!(val.span() => #_Some(#val)),
            None => quote!(#_None),
        };

        tokens.extend(opt_tokens);
    }
}

impl ToTokens for Origin<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (origin, span) = (self.0, self.1);
        let origin = origin.clone().into_normalized();
        define_spanned_export!(span => _uri);

        let path = origin.path().as_str();
        let query = Optional(origin.query().map(|q| q.as_str()));
        tokens.extend(quote_spanned! { span =>
            #_uri::Origin::const_new(#path, #query)
        });
    }
}

impl ToTokens for Absolute<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (absolute, span) = (self.0, self.1);
        define_spanned_export!(span => _uri);
        let absolute = absolute.clone().into_normalized();

        let scheme = absolute.scheme();
        let auth = Optional(absolute.authority().map(|a| Authority(a, span)));
        let path = absolute.path().as_str();
        let query = Optional(absolute.query().map(|q| q.as_str()));
        tokens.extend(quote_spanned! { span =>
            #_uri::Absolute::const_new(#scheme, #auth, #path, #query)
        });
    }
}

impl ToTokens for Authority<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (authority, span) = (self.0, self.1);
        define_spanned_export!(span => _uri);

        let user_info = Optional(authority.user_info());
        let host = authority.host();
        let port = Optional(authority.port());
        tokens.extend(quote_spanned! { span =>
            #_uri::Authority::const_new(#user_info, #host, #port)
        });
    }
}

impl ToTokens for Reference<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (reference, span) = (self.0, self.1);
        define_spanned_export!(span => _uri);
        let reference = reference.clone().into_normalized();

        let scheme = Optional(reference.scheme());
        let auth = Optional(reference.authority().map(|a| Authority(a, span)));
        let path = reference.path().as_str();
        let query = Optional(reference.query().map(|q| q.as_str()));
        let frag = Optional(reference.fragment().map(|f| f.as_str()));
        tokens.extend(quote_spanned! { span =>
            #_uri::Reference::const_new(#scheme, #auth, #path, #query, #frag)
        });
    }
}

impl ToTokens for Asterisk {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        define_spanned_export!(self.1 => _uri);
        tokens.extend(quote_spanned!(self.1 => #_uri::Asterisk));
    }
}
