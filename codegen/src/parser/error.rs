use syntax::ast::*;
use syntax::ext::base::{ExtCtxt, Annotatable};
use syntax::codemap::{Span, Spanned, dummy_spanned};

use rocket::http::Status;

use utils::*;
use super::Function;

/// This structure represents the parsed `error` attribute.
pub struct ErrorParams {
    pub annotated_fn: Function,
    pub code: Spanned<u16>,
}

impl ErrorParams {
    /// Parses the route attribute from the given decorator context. If the
    /// parse is not successful, this function exits early with the appropriate
    /// error message to the user.
    pub fn from(ecx: &mut ExtCtxt,
                sp: Span,
                meta_item: &MetaItem,
                annotated: &Annotatable)
                -> ErrorParams {
        let function = Function::from(annotated).unwrap_or_else(|item_sp| {
            span_err(ecx, sp, "this attribute can only be used on functions...");
            span_fatal(ecx, item_sp, "...but was applied to the item above.");
        });

        let meta_items = meta_item.meta_item_list().unwrap_or_else(|| {
            struct_span_fatal(ecx, sp, "incorrect use of attribute")
                .help("attributes in Rocket must have the form: #[name(...)]")
                .emit();
            span_fatal(ecx, sp, "malformed attribute");
        });

        if meta_items.len() < 1 {
            span_fatal(ecx, sp, "attribute requires the `code` parameter");
        } else if meta_items.len() > 1 {
            span_fatal(ecx, sp, "attribute can only have one `code` parameter");
        }

        ErrorParams {
            annotated_fn: function,
            code: parse_code(ecx, &meta_items[0])
        }
    }
}

fn parse_code(ecx: &ExtCtxt, meta_item: &NestedMetaItem) -> Spanned<u16> {
    let code_from_u128 = |n: Spanned<u128>| {
        if n.node < 400 || n.node > 599 {
            span_err(ecx, n.span, "code must be >= 400 and <= 599.");
            span(0, n.span)
        } else if Status::from_code(n.node as u16).is_none() {
            ecx.span_warn(n.span, "status code is unknown.");
            span(n.node as u16, n.span)
        } else {
            span(n.node as u16, n.span)
        }
    };

    let sp = meta_item.span();
    if let Some((name, lit)) = meta_item.name_value() {
        if name != &"code" {
            span_err(ecx, sp, "the first key, if any, must be 'code'");
        } else if let LitKind::Int(n, _) = lit.node {
            return code_from_u128(span(n, lit.span))
        } else {
            span_err(ecx, lit.span, "`code` value must be an integer")
        }
    } else if let Some(n) = meta_item.int_lit() {
        return code_from_u128(span(n, sp))
    } else {
        struct_span_err(ecx, sp, r#"expected `code = int` or an integer literal"#)
            .help(r#"you can specify the code directly as an integer,
                  e.g: #[error(404)], or as a key-value pair,
                  e.g: $[error(code = 404)]"#)
            .emit();
    }

    dummy_spanned(0)
}
