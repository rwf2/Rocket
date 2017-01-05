use std::str::FromStr;
use std::collections::HashSet;

use syntax::ast::*;
use syntax::ext::base::{ExtCtxt, Annotatable};
use syntax::codemap::{Span, Spanned, dummy_spanned};

use utils::{span, MetaItemExt, SpanExt, is_valid_ident};
use super::{Function, ParamIter};
use super::keyvalue::KVSpanned;
use rocket::http::{Method, ContentType};

/// This structure represents the parsed `route` attribute.
///
/// It contains all of the information supplied by the user and the span where
/// the user supplied the information. This structure can only be obtained by
/// calling the `RouteParams::from` function and passing in the entire decorator
/// environment.
#[derive(Debug)]
pub struct RouteParams {
    pub annotated_fn: Function,
    pub method: Spanned<Method>,
    pub path: Spanned<String>,
    pub data_param: Option<KVSpanned<Ident>>,
    pub query_param: Option<Spanned<Ident>>,
    pub format: Option<KVSpanned<ContentType>>,
    pub rank: Option<KVSpanned<isize>>,
}

impl RouteParams {
    /// Parses the route attribute from the given decorator context. If the
    /// parse is not successful, this function exits early with the appropriate
    /// error message to the user.
    pub fn from(ecx: &mut ExtCtxt,
                sp: Span,
                known_method: Option<Spanned<Method>>,
                meta_item: &MetaItem,
                annotated: &Annotatable)
                -> RouteParams {
        let function = Function::from(annotated).unwrap_or_else(|item_sp| {
            ecx.span_err(sp, "this attribute can only be used on functions...");
            ecx.span_fatal(item_sp, "...but was applied to the item above.");
        });

        let meta_items = meta_item.meta_item_list().unwrap_or_else(|| {
            ecx.struct_span_err(sp, "incorrect use of attribute")
                .help("attributes in Rocket must have the form: #[name(...)]")
                .emit();
            ecx.span_fatal(sp, "malformed attribute");
        });

        if meta_items.len() < 1 {
            ecx.span_fatal(sp, "attribute requires at least 1 parameter");
        }

        // Figure out the method. If it is known (i.e, because we're parsing a
        // helper attribute), use that method directly. Otherwise, try to parse
        // it from the list of meta items.
        let (method, attr_params) = match known_method {
            Some(method) => (method, meta_items),
            None => (parse_method(ecx, &meta_items[0]), &meta_items[1..])
        };

        if attr_params.len() < 1 {
            ecx.struct_span_err(sp, "attribute requires at least a path")
                .help(r#"example: #[get("/my/path")] or #[get(path = "/hi")]"#)
                .emit();
            ecx.span_fatal(sp, "malformed attribute");
        }

        // Parse the required path and optional query parameters.
        let (path, query) = parse_path(ecx, &attr_params[0]);

        // Parse all of the optional parameters.
        let mut seen_keys = HashSet::new();
        let (mut rank, mut data, mut format) = Default::default();
        for param in &attr_params[1..] {
            let kv_opt = kv_from_nested(&param);
            if kv_opt.is_none() {
                ecx.span_err(param.span(), "expected key = value");
                continue;
            }

            let kv = kv_opt.unwrap();
            match kv.key().as_str() {
                "rank" => rank = parse_opt(ecx, &kv, parse_rank),
                "data" => data = parse_opt(ecx, &kv, parse_data),
                "format" => format = parse_opt(ecx, &kv, parse_format),
                _ => {
                    let msg = format!("'{}' is not a known parameter", kv.key());
                    ecx.span_err(kv.span, &msg);
                    continue;
                }
            }

            if seen_keys.contains(kv.key()) {
                let msg = format!("{} was already defined", kv.key());
                ecx.struct_span_warn(param.span, &msg)
                   .note("the last declared value will be used")
                   .emit();
            } else {
                seen_keys.insert(kv.key().clone());
            }
        }

        // Sanity check: `data` should only be used with payload methods.
        if let Some(ref data_param) = data {
            if !method.node.supports_payload() {
                ecx.struct_span_err(data_param.span, "`data` route parameters \
                        can only be used with payload supporting methods")
                    .note(&format!("'{}' does not support payloads", method.node))
                    .emit();
            }
        }

        RouteParams {
            method: method,
            path: path,
            data_param: data,
            query_param: query,
            format: format,
            rank: rank,
            annotated_fn: function,
        }
    }

    pub fn path_params<'s, 'a, 'c: 'a>(&'s self,
                                   ecx: &'a ExtCtxt<'c>)
                                    -> ParamIter<'s, 'a, 'c> {
        ParamIter::new(ecx, self.path.node.as_str(), self.path.span.trim(1))
    }
}

fn is_valid_method(method: Method) -> bool {
    use rocket::http::Method::*;
    match method {
        Get | Put | Post | Delete | Head | Patch | Options => true,
        Trace | Connect => false
    }
}

pub fn kv_from_nested(item: &NestedMetaItem) -> Option<KVSpanned<LitKind>> {
    item.name_value().map(|(name, value)| {
        let k_span = item.span().shorten_to(name.as_str().len());
        KVSpanned {
            key: span(name.to_string(), k_span),
            value: value.clone(),
            span: item.span(),
        }
    })
}

fn param_string_to_ident(ecx: &ExtCtxt, s: Spanned<&str>) -> Option<Ident> {
    let string = s.node;
    if string.starts_with('<') && string.ends_with('>') {
        let param = &string[1..(string.len() - 1)];
        if is_valid_ident(param) {
            return Some(Ident::from_str(param));
        }

        ecx.span_err(s.span, "parameter name must be alphanumeric");
    } else {
        ecx.span_err(s.span, "parameters must start with '<' and end with '>'");
    }

    None
}

fn parse_method(ecx: &ExtCtxt, meta_item: &NestedMetaItem) -> Spanned<Method> {
    if let Some(word) = meta_item.word() {
        if let Ok(method) = Method::from_str(&word.name().as_str()) {
            if is_valid_method(method) {
                return span(method, word.span());
            }
        } else {
            let msg = format!("'{}' is not a valid HTTP method.", word.name());
            ecx.span_err(word.span(), &msg);
        }
    }

    // Fallthrough. Return default method.
    ecx.struct_span_err(meta_item.span, "expected a valid HTTP method")
        .help("valid methods are: GET, PUT, POST, DELETE, PATCH")
        .emit();

    return dummy_spanned(Method::Get);
}

fn parse_path(ecx: &ExtCtxt, meta_item: &NestedMetaItem) -> (Spanned<String>, Option<Spanned<Ident>>) {
    let from_string = |string: &str, sp: Span| {
        if let Some(q) = string.find('?') {
            let path = span(string[..q].to_string(), sp);
            let q_str = span(&string[(q + 1)..], sp);
            let query = param_string_to_ident(ecx, q_str).map(|i| span(i, sp));
            return (path, query);
        } else {
            return (span(string.to_string(), sp), None)
        }
    };

    let sp = meta_item.span();
    if let Some((name, lit)) = meta_item.name_value() {
        if name != &"path" {
            ecx.span_err(sp, "the first key, if any, must be 'path'");
        } else if let LitKind::Str(ref s, _) = lit.node {
            return from_string(&s.as_str(), lit.span);
        } else {
            ecx.span_err(lit.span, "`path` value must be a string")
        }
    } else if let Some(s) = meta_item.str_lit() {
        return from_string(&s.as_str(), sp);
    } else {
        ecx.struct_span_err(sp, r#"expected `path = string` or a path string"#)
            .help(r#"you can specify the path directly as a string, \
                  e.g: "/hello/world", or as a key-value pair, \
                  e.g: path = "/hello/world" "#)
            .emit();
    }

    (dummy_spanned("".to_string()), None)
}

fn parse_opt<O, T, F>(ecx: &ExtCtxt, kv: &KVSpanned<T>, f: F) -> Option<KVSpanned<O>>
    where F: Fn(&ExtCtxt, &KVSpanned<T>) -> O
{
    Some(kv.map_ref(|_| f(ecx, kv)))
}

fn parse_data(ecx: &ExtCtxt, kv: &KVSpanned<LitKind>) -> Ident {
    let mut ident = Ident::from_str("unknown");
    if let LitKind::Str(ref s, _) = *kv.value() {
        ident = Ident::from_str(&s.as_str());
        match param_string_to_ident(ecx, span(&s.as_str(), kv.value.span)) {
            Some(ident) => return ident,
            _ => { /* fall through if not an ident */ }
        }
    }

    let err_string = r#"`data` value must be a parameter, e.g: "<name>"`"#;
    ecx.struct_span_fatal(kv.span, err_string)
        .help(r#"data, if specified, must be a key-value pair where
              the key is `data` and the value is a string with a single
              parameter inside '<' '>'. e.g: data = "<user_form>""#)
        .emit();

    ident
}

fn parse_rank(ecx: &ExtCtxt, kv: &KVSpanned<LitKind>) -> isize {
    if let LitKind::Int(n, _) = *kv.value() {
        let max = isize::max_value();
        if n <= max as u128 {
            return n as isize;
        } else {
            let msg = format!("rank must be less than or equal to {}", max);
            ecx.span_err(kv.value.span, msg.as_str());
        }
    } else {
        ecx.struct_span_err(kv.span, r#"`rank` value must be an int"#)
            .help(r#"the rank, if specified, must be a key-value pair where
                  the key is `rank` and the value is an integer.
                  e.g: rank = 1, or e.g: rank = 10"#)
            .emit();
    }

    -1
}

fn parse_format(ecx: &ExtCtxt, kv: &KVSpanned<LitKind>) -> ContentType {
    if let LitKind::Str(ref s, _) = *kv.value() {
        if let Ok(ct) = ContentType::from_str(&s.as_str()) {
            if !ct.is_known() {
                let msg = format!("'{}' is not a known content-type", s);
                ecx.span_warn(kv.value.span, &msg);
            }

            return ct;
        } else {
            ecx.span_err(kv.value.span, "malformed content-type");
        }
    }

    ecx.struct_span_err(kv.span, r#"`format` must be a "content/type""#)
        .help(r#"format, if specified, must be a key-value pair where
              the key is `format` and the value is a string representing the
              content-type accepted. e.g: format = "application/json""#)
        .emit();

    ContentType::Any
}
