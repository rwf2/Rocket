#![allow(unused_imports)] // FIXME: Why is this coming from quote_tokens?

use std::mem::transmute;
use std::collections::HashMap;

use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::print::pprust::{stmt_to_string};
use syntax::ast::{ItemKind, Expr, MetaItem, Mutability, VariantData, Ident};
use syntax::ast::{StructField, GenericParam};
use syntax::codemap::Span;
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use syntax_ext::deriving::generic::MethodDef;
use syntax_ext::deriving::generic::{StaticStruct, Substructure, TraitDef, ty};
use syntax_ext::deriving::generic::combine_substructure as c_s;

use utils::{strip_ty_lifetimes, is_valid_ident, SpanExt};

static ONLY_STRUCTS_ERR: &'static str = "`FromForm` can only be derived for \
    structures with named fields.";
static PRIVATE_LIFETIME: &'static str = "'rocket";

fn struct_lifetime(ecx: &mut ExtCtxt, item: &Annotatable, sp: Span) -> Option<String> {
    match *item {
        Annotatable::Item(ref item) => match item.node {
            ItemKind::Struct(_, ref generics) => {
                let mut lifetimes = generics.params.iter()
                    .filter_map(|p| match *p {
                        GenericParam::Lifetime(ref def) => Some(def),
                        _ => None
                    });

                let lifetime = lifetimes.next().map(|d| d.lifetime.ident.to_string());
                if lifetimes.next().is_some() {
                    ecx.span_err(generics.span, "cannot have more than one \
                        lifetime parameter when deriving `FromForm`.");
                }

                lifetime
            },
            _ => ecx.span_fatal(sp, ONLY_STRUCTS_ERR)
        },
        _ => ecx.span_fatal(sp, ONLY_STRUCTS_ERR)
    }
}

// TODO: Use proper logging to emit the error messages.
pub fn from_form_derive(
    ecx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotated: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let struct_lifetime = struct_lifetime(ecx, annotated, span);
    let (lifetime_var, trait_generics) = match struct_lifetime {
        Some(ref lifetime) => (Some(lifetime.as_str()), ty::LifetimeBounds::empty()),
        None => (Some(PRIVATE_LIFETIME), ty::LifetimeBounds {
                lifetimes: vec![(PRIVATE_LIFETIME, vec![])],
                bounds: vec![]
            })
    };

    // The error type in the derived implementation.
    let error_type = ty::Ty::Literal(ty::Path::new_(vec!["rocket", "Error"],
                                                    None, vec![], ty::PathKind::Global));

    let trait_def = TraitDef {
        is_unsafe: false,
        supports_unions: false,
        span: span,
        // We add these attribute because some `FromFormValue` implementations
        // can't fail. This is indicated via the `!` type. Rust checks if a
        // match is made with something of that type, and since we always emit
        // an `Err` match, we'll get this lint warning.
        attributes: vec![quote_attr!(ecx, #[allow(unreachable_code, unreachable_patterns)])],
        path: ty::Path::new_(
            vec!["rocket", "request", "FromForm"],
            lifetime_var,
            vec![],
            ty::PathKind::Global,
        ),
        additional_bounds: Vec::new(),
        generics: trait_generics,
        methods: vec![
            MethodDef {
                name: "from_form",
                generics: ty::LifetimeBounds::empty(),
                explicit_self: None,
                args: vec![
                    ty::Ptr(
                        Box::new(ty::Literal(ty::Path::new_(
                            vec!["rocket", "request", "FormItems"],
                            lifetime_var,
                            vec![],
                            ty::PathKind::Global,
                        ))),
                        ty::Borrowed(None, Mutability::Mutable)
                    ),
                    ty::Literal(ty::Path::new_local("bool")),
                ],
                ret_ty: ty::Literal(ty::Path::new_(
                    vec!["result", "Result"],
                    None,
                    vec![
                        Box::new(ty::Ty::Self_),
                        Box::new(error_type.clone())
                    ],
                    ty::PathKind::Std,
                )),
                attributes: vec![],
                is_unsafe: false,
                combine_substructure: c_s(Box::new(from_form_substructure)),
                unify_fieldless_variants: false,
            }
        ],
        associated_types: vec![
            (Ident::from_str("Error"), error_type.clone())
        ],
    };

    trait_def.expand(ecx, meta_item, annotated, push);
}

fn is_valid_field_name(name: &str) -> bool {
    // The HTML5 spec (4.10.18.1) says 'isindex' is not allowed.
    if name == "isindex" || name.is_empty() {
        return false
    }

    // We allow all visible ASCII characters except '&', '=', and '?' since we
    // use those as control characters for parsing.
    name.chars()
        .all(|c| (c >= ' ' && c <= '~') && c != '&' && c != '=' && c != '?')
}

pub fn extract_field_ident_name(ecx: &ExtCtxt, struct_field: &StructField)
        -> (Ident, String, Span) {
    let ident = match struct_field.ident {
        Some(ident) => ident,
        None => ecx.span_fatal(struct_field.span, ONLY_STRUCTS_ERR)
    };

    let field_attrs: Vec<_> = struct_field.attrs.iter()
        .filter(|attr| attr.check_name("form"))
        .collect();

    let default = |ident: Ident| (ident, ident.to_string(), struct_field.span);
    if field_attrs.len() == 0 {
        return default(ident);
    } else if field_attrs.len() > 1 {
        ecx.span_err(struct_field.span, "only a single #[form(..)] \
            attribute can be applied to a given struct field at a time");
        return default(ident);
    }

    let field_attr = field_attrs[0];
    ::syntax::attr::mark_known(&field_attr);
    if !field_attr.meta_item_list().map_or(false, |l| l.len() == 1) {
        ecx.struct_span_err(field_attr.span, "incorrect use of attribute")
            .help(r#"the `form` attribute must have the form: #[form(field = "..")]"#)
            .emit();
        return default(ident);
    }

    let inner_item = &field_attr.meta_item_list().unwrap()[0];
    if !inner_item.check_name("field") {
        ecx.struct_span_err(inner_item.span, "invalid `form` attribute contents")
            .help(r#"only the 'field' key is supported: #[form(field = "..")]"#)
            .emit();
        return default(ident);
    }

    if !inner_item.is_value_str() {
        ecx.struct_span_err(inner_item.span, "invalid `field` in attribute")
            .help(r#"the `form` attribute must have the form: #[form(field = "..")]"#)
            .emit();
        return default(ident);
    }

    let name = inner_item.value_str().unwrap().as_str().to_string();
    let sp = inner_item.span.shorten_upto(name.len() + 2);
    if !is_valid_field_name(&name) {
        ecx.struct_span_err(sp, "invalid form field name")
            .help("field names must be visible ASCII without '&', '=', or '?'")
            .emit();
    }

    (ident, name, sp)
}

fn from_form_substructure(cx: &mut ExtCtxt, trait_span: Span, substr: &Substructure) -> P<Expr> {
    // Check that we specified the methods to the argument correctly.
    const EXPECTED_ARGS: usize = 2;
    let (items_arg, strict_arg) = if substr.nonself_args.len() == EXPECTED_ARGS {
        (&substr.nonself_args[0], &substr.nonself_args[1])
    } else {
        let msg = format!("incorrect number of arguments in `from_form_string`: \
            expected {}, found {}", EXPECTED_ARGS, substr.nonself_args.len());
        cx.span_bug(trait_span, msg.as_str());
    };

    debug!("arguments are: {:?}, {:?}", items_arg, strict_arg);

    // Ensure the the fields are from a 'StaticStruct' and extract them.
    let fields = match *substr.fields {
        StaticStruct(var_data, _) => match *var_data {
            VariantData::Struct(ref fields, _) => fields,
            _ => cx.span_fatal(trait_span, ONLY_STRUCTS_ERR)
        },
        _ => cx.span_bug(trait_span, "impossible substructure in `from_form`")
    };

    // Vec of (ident: Ident, type: Ty, name: String), one for each field.
    let mut names = HashMap::new();
    let mut fields_info = vec![];
    for field in fields {
        let (ident, name, span) = extract_field_ident_name(cx, field);
        let stripped_ty = strip_ty_lifetimes(field.ty.clone());

        if let Some(sp) = names.get(&name).map(|sp| *sp) {
            cx.struct_span_err(span, "field with duplicate name")
                .span_note(sp, "original was declared here")
                .emit();
        } else {
            names.insert(name.clone(), span);
        }

        fields_info.push((ident, stripped_ty, name));
    }

    debug!("Fields, types, attrs: {:?}", fields_info);
    let mut stmts = Vec::new();

    // The thing to do when we wish to exit with an error.
    let return_err_stmt = quote_tokens!(cx,
        return Err(::rocket::Error::BadParse)
    );

    // Generate the let bindings for parameters that will be unwrapped and
    // placed into the final struct. They start out as `None` and are changed
    // to Some when a parse completes, or some default value if the parse was
    // unsuccessful and default() returns Some.
    for &(ref ident, ref ty, _) in &fields_info {
        stmts.push(quote_stmt!(cx,
            let mut $ident: ::std::option::Option<$ty> = None;
        ).unwrap());
    }

    // Generating an arm for each struct field. This matches against the key and
    // tries to parse the value according to the type.
    let mut arms = vec![];
    for &(ref ident, _, ref name) in &fields_info {
        arms.push(quote_tokens!(cx,
            $name => {
                let __r = ::rocket::http::RawStr::from_str(__v);
                $ident = match ::rocket::request::FromFormValue::from_form_value(__r) {
                    Ok(__v) => Some(__v),
                    Err(__e) => {
                        println!("    => Error parsing form val '{}': {:?}",
                                 $name, __e);
                        $return_err_stmt
                    }
                };
            },
        ));
    }

    // The actual match statement. Iterate through all of the fields in the form
    // and use the $arms generated above.
    stmts.push(quote_stmt!(cx,
        for (__k, __v) in $items_arg {
            match __k.as_str() {
                $arms
                _ => {
                    // If we're parsing strictly, emit an error for everything
                    // the user hasn't asked for. Keep synced with 'preprocess'.
                    if $strict_arg && __k != "_method" {
                        println!("    => {}={} has no matching field in struct.",
                                 __k, __v);
                        $return_err_stmt
                    }
                }
           };
       }
    ).unwrap());

    // This looks complicated but just generates the boolean condition checking
    // that each parameter actually is Some() or has a default value.
    let mut failure_conditions = vec![];

    for &(ref ident, ref ty, _) in (&fields_info).iter() {
        failure_conditions.push(quote_tokens!(cx,
            if $ident.is_none() &&
                <$ty as ::rocket::request::FromFormValue>::default().is_none() {
                println!("    => '{}' did not parse.", stringify!($ident));
                $return_err_stmt;
            }
        ));
    }

    // The fields of the struct, which are just the let bindings declared above
    // or the default value.
    let mut result_fields = vec![];
    for &(ref ident, ref ty, _) in &fields_info {
        result_fields.push(quote_tokens!(cx,
            $ident: $ident.unwrap_or_else(||
                <$ty as ::rocket::request::FromFormValue>::default().unwrap()
            ),
        ));
    }

    // The final block: check the error conditions, and if all is well, return
    // the structure.
    let self_ident = substr.type_ident;
    let final_block = quote_block!(cx, {
        $failure_conditions

        Ok($self_ident { $result_fields })
    });

    stmts.extend(final_block.into_inner().stmts);

    debug!("Form statements:");
    for stmt in &stmts {
        debug!("{:?}", stmt_to_string(stmt));
    }

    cx.expr_block(cx.block(trait_span, stmts))
}


