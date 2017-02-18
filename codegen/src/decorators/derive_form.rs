#![allow(unused_imports)] // FIXME: Why is this coming from quote_tokens?

use std::mem::transmute;

use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::print::pprust::{stmt_to_string};
use syntax::ast::{ItemKind, Expr, MetaItem, Mutability, VariantData, Ident};
use syntax::codemap::Span;
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use syntax_ext::deriving::generic::MethodDef;
use syntax_ext::deriving::generic::{StaticStruct, Substructure, TraitDef, ty};
use syntax_ext::deriving::generic::combine_substructure as c_s;

use utils::strip_ty_lifetimes;

static ONLY_STRUCTS_ERR: &'static str = "`FromForm` can only be derived for \
    structures with named fields.";
static PRIVATE_LIFETIME: &'static str = "'rocket";

fn get_struct_lifetime(ecx: &mut ExtCtxt, item: &Annotatable, span: Span)
        -> Option<&'static str> {
    match *item {
        Annotatable::Item(ref item) => match item.node {
            ItemKind::Struct(_, ref generics) => {
                match generics.lifetimes.len() {
                    0 => None,
                    1 => {
                        let lifetime = generics.lifetimes[0].lifetime;
                        // According to the documentation, this is safe:
                        //  Because the interner lives for the life of the
                        //  thread, this can be safely treated as an immortal
                        //  string, as long as it never crosses between threads.
                        let lifetime_name: &'static str =
                            unsafe { transmute(&*lifetime.name.as_str()) };
                        Some(lifetime_name)
                    }
                    _ => {
                        ecx.span_err(item.span, "cannot have more than one \
                            lifetime parameter when deriving `FromForm`.");
                        None
                    }
                }
            },
            _ => ecx.span_fatal(span, ONLY_STRUCTS_ERR)
        },
        _ => ecx.span_fatal(span, ONLY_STRUCTS_ERR)
    }
}

// TODO: Use proper logging to emit the error messages.
pub fn from_form_derive(ecx: &mut ExtCtxt, span: Span, meta_item: &MetaItem,
          annotated: &Annotatable, push: &mut FnMut(Annotatable)) {
    let struct_lifetime = get_struct_lifetime(ecx, annotated, span);
    let (lifetime_var, trait_generics) = match struct_lifetime {
        lifetime@Some(_) => (lifetime, ty::LifetimeBounds::empty()),
        None => (Some(PRIVATE_LIFETIME), ty::LifetimeBounds {
                lifetimes: vec![(PRIVATE_LIFETIME, vec![])],
                bounds: vec![]
            })
    };

    // The error type in the derived implementation.
    let error_type = ty::Ty::Literal(ty::Path::new(vec!["rocket", "Error"]));

    let trait_def = TraitDef {
        is_unsafe: false,
        supports_unions: false,
        span: span,
        attributes: Vec::new(),
        path: ty::Path {
            path: vec!["rocket", "request", "FromForm"],
            lifetime: lifetime_var,
            params: vec![],
            global: true,
        },
        additional_bounds: Vec::new(),
        generics: trait_generics,
        methods: vec![
            MethodDef {
                name: "from_form_items",
                generics: ty::LifetimeBounds::empty(),
                explicit_self: None,
                args: vec![
                    ty::Ptr(
                        Box::new(ty::Literal(ty::Path {
                            path: vec!["rocket", "request", "FormItems"],
                            lifetime: lifetime_var,
                            params: vec![],
                            global: true
                        })),
                        ty::Borrowed(None, Mutability::Mutable)
                    )
                ],
                ret_ty: ty::Ty::Literal(
                    ty::Path {
                        path: vec!["std", "result", "Result"],
                        lifetime: None,
                        params: vec![
                            Box::new(ty::Ty::Self_),
                            Box::new(error_type.clone())
                        ],
                        global: true,
                    }
                ),
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

fn from_form_substructure(cx: &mut ExtCtxt, trait_span: Span, substr: &Substructure) -> P<Expr> {
    // Check that we specified the methods to the argument correctly.
    const EXPECTED_ARGS: usize = 1;
    let arg = if substr.nonself_args.len() == EXPECTED_ARGS {
        &substr.nonself_args[0]
    } else {
        let msg = format!("incorrect number of arguments in `from_form_string`: \
            expected {}, found {}", EXPECTED_ARGS, substr.nonself_args.len());
        cx.span_bug(trait_span, msg.as_str());
    };

    debug!("argument is: {:?}", arg);

    // Ensure the the fields are from a 'StaticStruct' and extract them.
    let fields = match *substr.fields {
        StaticStruct(var_data, _) => match *var_data {
            VariantData::Struct(ref fields, _) => fields,
            _ => cx.span_fatal(trait_span, ONLY_STRUCTS_ERR)
        },
        _ => cx.span_bug(trait_span, "impossible substructure in `from_form`")
    };

    // Create a vector of (ident, type) pairs, one for each field in struct.
    let mut fields_and_types = vec![];
    for field in fields {
        let ident = match field.ident {
            Some(ident) => ident,
            None => cx.span_fatal(trait_span, ONLY_STRUCTS_ERR)
        };

        let stripped_ty = strip_ty_lifetimes(field.ty.clone());
        fields_and_types.push((ident, stripped_ty));
    }

    debug!("Fields and types: {:?}", fields_and_types);
    let mut stmts = Vec::new();

    // The thing to do when we wish to exit with an error.
    let return_err_stmt = quote_tokens!(cx,
        return Err(::rocket::Error::BadParse)
    );

    // Generate the let bindings for parameters that will be unwrapped and
    // placed into the final struct. They start out as `None` and are changed
    // to Some when a parse completes, or some default value if the parse was
    // unsuccessful and default() returns Some.
    for &(ref ident, ref ty) in &fields_and_types {
        stmts.push(quote_stmt!(cx,
            let mut $ident: ::std::option::Option<$ty> = None;
        ).unwrap());
    }

    // Generating an arm for each struct field. This matches against the key and
    // tries to parse the value according to the type.
    let mut arms = vec![];
    for &(ref ident, _) in &fields_and_types {
        let ident_string = ident.to_string();
        let id_str = ident_string.as_str();
        arms.push(quote_tokens!(cx,
            $id_str => {
                $ident = match ::rocket::request::FromFormValue::from_form_value(v) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        println!("    => Error parsing form val '{}': {:?}",
                                 $id_str, e);
                        $return_err_stmt
                    }
                };
            },
        ));
    }

    // The actual match statement. Iterate through all of the fields in the form
    // and use the $arms generated above.
    stmts.push(quote_stmt!(cx,
        for (k, v) in $arg {
            match k {
                $arms
                field if field == "_method" => {
                    /* This is a Rocket-specific field. If the user hasn't asked
                     * for it, just let it go by without error. This should stay
                     * in sync with Rocket::preprocess. */
                }
                _ => {
                    println!("    => {}={} has no matching field in struct.",
                             k, v);
                    $return_err_stmt
                }
           };
       }
    ).unwrap());

    // This looks complicated but just generates the boolean condition checking
    // that each parameter actually is Some() or has a default value.
    let mut failure_conditions = vec![];

    // Start with `false` in case there are no fields.
    failure_conditions.push(quote_tokens!(cx, false));

    for &(ref ident, ref ty) in (&fields_and_types).iter() {
        // Pushing an "||" (or) between every condition.
        failure_conditions.push(quote_tokens!(cx, ||));

        failure_conditions.push(quote_tokens!(cx,
            if $ident.is_none() &&
                <$ty as ::rocket::request::FromFormValue>::default().is_none() {
                println!("    => '{}' did not parse.", stringify!($ident));
                true
            } else { false }
        ));
    }

    // The fields of the struct, which are just the let bindings declared above
    // or the default value.
    let mut result_fields = vec![];
    for &(ref ident, ref ty) in &fields_and_types {
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
        if $failure_conditions {
            $return_err_stmt;
        }

        Ok($self_ident { $result_fields })
    });

    stmts.extend(final_block.unwrap().stmts);

    debug!("Form statements:");
    for stmt in &stmts {
        debug!("{:?}", stmt_to_string(stmt));
    }

    cx.expr_block(cx.block(trait_span, stmts))
}

