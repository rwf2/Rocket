use devise::{*, ext::{TypeExt, SpanDiagnosticExt, GenericsExt, Split2, quote_respanned}};
use proc_macro2::TokenStream;

use crate::exports::*;
use crate::derive::form_field::{*, FieldName::*};

// F: fn(field_ty: Ty, field_context: Expr)
fn fields_map<F>(fields: Fields<'_>, map_f: F) -> Result<TokenStream>
    where F: Fn(&syn::Type, &syn::Expr) -> TokenStream
{
    let mut matchers = vec![];
    for field in fields.iter() {
        let (ident, ty) = (field.ident(), field.stripped_ty());
        let field_context = quote_spanned!(ty.span() => {
            let _o = __c.__opts;
            __c.#ident.get_or_insert_with(|| <#ty as #_form::FromForm<'__f>>::init(_o))
        });

        let field_names = field.field_names()?;
        let field_context = syn::parse2(field_context).expect("valid expr");
        let push = map_f(&ty, &field_context);
        let field_matchers = field_names.iter().map(|f| match f {
            Cased(name) => quote!(#name => { #push }),
            Uncased(name) => quote!(__n if __n.as_uncased() == #name => { #push }),
        });

        matchers.extend(field_matchers);
    }

    Ok(quote! {
        __c.__parent = __f.name.parent();

        match __f.name.key_lossy().as_str() {
            #(#matchers,)*
            _k if _k == "_method" || !__c.__opts.strict => { /* ok */ },
            _ => __c.__errors.push(__f.unexpected()),
        }
    })
}

fn context_type(input: Input<'_>) -> (TokenStream, Option<syn::WhereClause>) {
    let mut gen = input.generics().clone();

    let lifetime = syn::parse_quote!('__f);
    if !gen.replace_lifetime(0, &lifetime) {
        gen.insert_lifetime(syn::LifetimeDef::new(lifetime.clone()));
    }

    let span = input.ident().span();
    gen.add_type_bound(syn::parse_quote!(#_form::FromForm<#lifetime>));
    gen.add_type_bound(syn::TypeParamBound::from(lifetime));
    let (_, ty_gen, where_clause) = gen.split_for_impl();
    (quote_spanned!(span => FromFormGeneratedContext #ty_gen), where_clause.cloned())
}


pub fn derive_from_form(input: proc_macro::TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl<'__f> #_form::FromForm<'__f>))
        // NOTE: If support is widened, fix `FieldExt::ident()` `expect()`.
        .support(Support::NamedStruct | Support::Lifetime | Support::Type)
        .replace_generic(0, 0)
        .type_bound(quote!(#_form::FromForm<'__f> + '__f))
        .validator(ValidatorBuild::new()
            .input_validate(|_, i| match i.generics().lifetimes().enumerate().last() {
                Some((i, lt)) if i >= 1 => Err(lt.span().error("only one lifetime is supported")),
                _ => Ok(())
            })
            .fields_validate(|_, fields| {
                if fields.is_empty() {
                    return Err(fields.span().error("at least one field is required"));
                }

                if let Some(d) = first_duplicate(fields.iter(), |f| f.field_names())? {
                    let (field_a_i, field_a, name_a) = d.0;
                    let (field_b_i, field_b, name_b) = d.1;

                    if field_a_i == field_b_i {
                        return Err(field_a.error("field has conflicting names")
                            .span_note(name_a, "this field name...")
                            .span_note(name_b, "...conflicts with this field name"));
                    }

                    return Err(name_b.error("field name conflicts with previous name")
                        .span_help(field_b, "declared in this field")
                        .span_note(field_a, "previous field with conflicting name"));
                }

                Ok(())
            })
        )
        .outer_mapper(MapperBuild::new()
            .try_input_map(|mapper, input|  {
                let (ctxt_ty, where_clause) = context_type(input);
                let output = mapper::input_default(mapper, input)?;
                Ok(quote! {
                    /// Rocket generated FormForm context.
                    #[doc(hidden)]
                    pub struct #ctxt_ty #where_clause {
                        __opts: #_form::Options,
                        __errors: #_form::Errors<'__f>,
                        __parent: #_Option<&'__f #_form::Name>,
                        #output
                    }
                })
            })
            .try_fields_map(|m, f| mapper::fields_null(m, f))
            .field_map(|_, field| {
                let (ident, mut ty) = (field.ident(), field.stripped_ty());
                ty.replace_lifetimes(syn::parse_quote!('__f));
                let field_ty = quote_respanned!(ty.span() =>
                    #_Option<<#ty as #_form::FromForm<'__f>>::Context>
                );

                quote_spanned!(ty.span() => #ident: #field_ty,)
            })
        )
        .outer_mapper(quote! {
            #[allow(unused_imports)]
            use #_http::uncased::AsUncased;
        })
        .outer_mapper(quote!(#[rocket::async_trait]))
        .inner_mapper(MapperBuild::new()
            .try_input_map(|mapper, input| {
                let (ctxt_ty, _) = context_type(input);
                let output = mapper::input_default(mapper, input)?;
                Ok(quote! {
                    type Context = #ctxt_ty;

                    fn init(__opts: #_form::Options) -> Self::Context {
                        Self::Context {
                            __opts,
                            __errors: #_form::Errors::new(),
                            __parent: #_None,
                            #output
                        }
                    }
                })
            })
            .try_fields_map(|m, f| mapper::fields_null(m, f))
            .field_map(|_, field| {
                let ident = field.ident.as_ref().expect("named");
                let ty = field.ty.with_stripped_lifetimes();
                quote_spanned!(ty.span() =>
                    #ident: #_None,
                    // #ident: <#ty as #_form::FromForm<'__f>>::init(__opts),
                )
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn push_value(__c: &mut Self::Context, __f: #_form::ValueField<'__f>) {
                    #output
                }
            })
            .try_fields_map(|_, f| fields_map(f, |ty, ctxt| quote_spanned!(ty.span() => {
                <#ty as #_form::FromForm<'__f>>::push_value(#ctxt, __f.shift());
            })))
        )
        .inner_mapper(MapperBuild::new()
            .try_input_map(|mapper, input| {
                let (ctxt_ty, _) = context_type(input);
                let output = mapper::input_default(mapper, input)?;
                Ok(quote! {
                    async fn push_data(
                        __c: &mut #ctxt_ty,
                        __f: #_form::DataField<'__f, '_>
                    ) {
                        #output
                    }
                })
            })
            // Without the `let _fut`, we get a wild lifetime error. It don't
            // make no sense, Rust async/await, it don't make no sense.
            .try_fields_map(|_, f| fields_map(f, |ty, ctxt| quote_spanned!(ty.span() => {
                let _fut = <#ty as #_form::FromForm<'__f>>::push_data(#ctxt, __f.shift());
                _fut.await;
            })))
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn finalize(mut __c: Self::Context) -> #_Result<Self, #_form::Errors<'__f>> {
                    #[allow(unused_imports)]
                    use #_form::validate::*;

                    #output
                }
            })
            .try_fields_map(|mapper, fields| {
                let finalize_field = fields.iter()
                    .map(|f| mapper.map_field(f))
                    .collect::<Result<Vec<TokenStream>>>()?;

                let ident: Vec<_> = fields.iter()
                    .map(|f| f.ident().clone())
                    .collect();

                let o = syn::Ident::new("__o", fields.span());
                let (_ok, _some, _err, _none) = (_Ok, _Some, _Err, _None);
                let (name_view, validate) = fields.iter()
                    .map(|f| (f.name_view().unwrap(), validators(f, &o, false).unwrap()))
                    .map(|(nv, vs)| vs.map(move |v| (nv.clone(), v)))
                    .flatten()
                    .split2();

                Ok(quote_spanned! { fields.span() =>
                    #(let #ident = match #finalize_field {
                        #_ok(#ident) => #_some(#ident),
                        #_err(_e) => { __c.__errors.extend(_e); #_none }
                    };)*

                    if !__c.__errors.is_empty() {
                        return #_Err(__c.__errors);
                    }

                    let #o = Self { #(#ident: #ident.unwrap()),* };

                    #(
                        if let #_err(_e) = #validate {
                            __c.__errors.extend(_e.with_name(#name_view));
                        }
                    )*

                    if !__c.__errors.is_empty() {
                        return #_Err(__c.__errors);
                    }

                    Ok(#o)
                })
            })
            .try_field_map(|_, f| {
                let (ident, ty, name_view) = (f.ident(), f.stripped_ty(), f.name_view()?);
                let validator = validators(f, &ident, true)?;
                let default = default(f)?
                    .unwrap_or_else(|| quote_spanned!(ty.span() => {
                        <#ty as #_form::FromForm<'__f>>::default(__opts)
                    }));

                let _err = _Err;
                Ok(quote_spanned! { ty.span() => {
                    let __name = #name_view;
                    let __opts = __c.__opts;
                    __c.#ident
                        .map(<#ty as #_form::FromForm<'__f>>::finalize)
                        .unwrap_or_else(|| {
                            #default.ok_or_else(|| #_form::ErrorKind::Missing.into())
                        })
                        .and_then(|#ident| {
                            let mut __es = #_form::Errors::new();
                            #(if let #_err(__e) = #validator { __es.extend(__e); })*

                            match __es.is_empty() {
                                true => #_Ok(#ident),
                                false => #_Err(__es)
                            }
                        })
                        .map_err(|__e| __e.with_name(__name))
                        .map_err(|__e| match __e.is_empty() {
                            true => #_form::ErrorKind::Unknown.into(),
                            false => __e,
                        })
                }})
            })
        )
        .to_tokens()
}
