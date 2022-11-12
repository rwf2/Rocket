use proc_macro2::TokenStream;
use devise::ext::{TypeExt, SpanDiagnosticExt, GenericsExt, quote_respanned};
use syn::parse::Parser;
use devise::*;

use crate::exports::*;
use crate::derive::form_field::{*, FieldName::*};
use crate::syn_ext::{GenericsExt as _, TypeExt as _};

type WherePredicates = syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>;

// F: fn(field_ty: Ty, field_context: Expr)
fn fields_map<F>(fields: Fields<'_>, map_f: F) -> Result<TokenStream>
    where F: Fn(&syn::Type, &syn::Expr) -> TokenStream
{
    let mut matchers = vec![];
    for field in fields.iter() {
        let (ident, ty) = (field.context_ident(), field.stripped_ty());
        let field_context: syn::Expr = syn::parse2(quote_spanned!(ty.span() => {
            let __o = __c.__opts;
            __c.#ident.get_or_insert_with(|| <#ty as #_form::FromForm<'r>>::init(__o))
        })).expect("form context expression");

        let push = map_f(&ty, &field_context);
        if fields.are_unnamed() {
            // If we have unnamed fields, then we have exactly one by virtue of
            // the earlier validation. Push directly to it and return.
            return Ok(quote_spanned!(ident.span() =>
                __c.__parent = __f.name.parent();
                 #push
            ));
        }

        matchers.extend(field.field_names()?.into_iter().map(|f| match f {
            Cased(name) => quote!(#name => { #push }),
            Uncased(name) => quote!(__n if __n.as_uncased() == #name => { #push }),
        }));
    }

    Ok(quote! {
        __c.__parent = __f.name.parent();

        match __f.name.key_lossy().as_str() {
            #(#matchers,)*
            __k if __k == "_method" || !__c.__opts.strict => { /* ok */ },
            _ => __c.__errors.push(__f.unexpected()),
        }
    })
}

fn generic_bounds_tokens(input: Input<'_>) -> Result<TokenStream> {
    MapperBuild::new()
        .try_enum_map(|m, e| mapper::enum_null(m, e))
        .try_fields_map(|_, fields| {
            let generic_idents = fields.parent.input().generics().type_idents();

            let bounds = fields.iter()
                .filter(|f| !f.ty.is_concrete(&generic_idents))
                .map(|f| f.ty.with_replaced_lifetimes(syn::Lifetime::new("'r", f.ty.span())))
                .map(|ty| quote_spanned!(ty.span() => #ty: #_form::FromForm<'r>));

            Ok(quote!(#(#bounds),*))
        })
        .map_input(input)
}

fn generic_bounds(input: Input<'_>) -> Result<WherePredicates> {
    Ok(WherePredicates::parse_terminated.parse2(generic_bounds_tokens(input)?)?)
}

fn context_type(input: Input<'_>) -> Result<(TokenStream, syn::Generics)> {
    let mut gen = input.generics().clone();

    let lifetime = syn::parse_quote!('r);
    if !gen.replace_lifetime(0, &lifetime) {
        gen.insert_lifetime(syn::LifetimeDef::new(lifetime.clone()));
    }

    gen.add_where_predicates(generic_bounds(input)?);
    let ty = quote_spanned!(input.ident().span() => FromFormGeneratedContext);
    Ok((ty, gen))
}

pub fn derive_from_form(input: proc_macro::TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl<'r> #_form::FromForm<'r>))
        .support(Support::Struct | Support::Lifetime | Support::Type)
        .replace_generic(0, 0)
        .type_bound_mapper(MapperBuild::new().try_input_map(|_, i| generic_bounds_tokens(i)))
        .validator(ValidatorBuild::new()
            .input_validate(|_, i| match i.generics().lifetimes().enumerate().last() {
                Some((i, lt)) if i >= 1 => Err(lt.span().error("only one lifetime is supported")),
                _ => Ok(())
            })
            .fields_validate(|_, fields| {
                if fields.is_empty() {
                    return Err(fields.span().error("at least one field is required"));
                } else if fields.are_unnamed() && fields.count() != 1 {
                    return Err(fields.span().error("tuple struct must have exactly one field"));
                } else if let Some(d) = first_duplicate(fields.iter(), |f| f.field_names())? {
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
                let vis = input.vis();
                let (ctxt_ty, gen) = context_type(input)?;
                let (impl_gen, _, where_clause)  = gen.split_for_impl();
                let output = mapper::input_default(mapper, input)?;
                Ok(quote_spanned! { input.span() =>
                    /// Rocket generated FormForm context.
                    #[doc(hidden)]
                    #[allow(private_in_public)]
                    #vis struct #ctxt_ty #impl_gen #where_clause {
                        __opts: #_form::Options,
                        __errors: #_form::Errors<'r>,
                        __parent: #_Option<&'r #_form::Name>,
                        #output
                    }
                })
            })
            .try_fields_map(|m, f| mapper::fields_null(m, f))
            .field_map(|_, field| {
                let ident = field.context_ident();
                let mut ty = field.stripped_ty();
                ty.replace_lifetimes(syn::parse_quote!('r));
                let field_ty = quote_respanned!(ty.span() =>
                    #_Option<<#ty as #_form::FromForm<'r>>::Context>
                );

                quote_spanned!(ty.span() => #ident: #field_ty,)
            })
        )
        .outer_mapper(quote! {
            #[allow(unused_imports)]
            use #_http::uncased::AsUncased;
        })
        .outer_mapper(quote!(#[allow(private_in_public)]))
        .outer_mapper(quote!(#[rocket::async_trait]))
        .inner_mapper(MapperBuild::new()
            .try_input_map(|mapper, input| {
                let (ctxt_ty, gen) = context_type(input)?;
                let (_, ty_gen, _) = gen.split_for_impl();
                let output = mapper::input_default(mapper, input)?;
                Ok(quote! {
                    type Context = #ctxt_ty #ty_gen;

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
                let ident = field.context_ident();
                let ty = field.ty.with_stripped_lifetimes();
                quote_spanned!(ty.span() => #ident: #_None,)
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn push_value(__c: &mut Self::Context, __f: #_form::ValueField<'r>) {
                    #output
                }
            })
            .try_fields_map(|_, f| fields_map(f, |ty, ctxt| quote_spanned!(ty.span() => {
                <#ty as #_form::FromForm<'r>>::push_value(#ctxt, __f.shift());
            })))
        )
        .inner_mapper(MapperBuild::new()
            .try_input_map(|mapper, input| {
                let (ctxt_ty, gen) = context_type(input)?;
                let (_, ty_gen, _) = gen.split_for_impl();
                let output = mapper::input_default(mapper, input)?;
                Ok(quote! {
                    async fn push_data(
                        __c: &mut #ctxt_ty #ty_gen,
                        __f: #_form::DataField<'r, '_>
                    ) {
                        #output
                    }
                })
            })
            // Without the `let _fut`, we get a wild lifetime error. It don't
            // make no sense, Rust async/await: it don't make no sense.
            .try_fields_map(|_, f| fields_map(f, |ty, ctxt| quote_spanned!(ty.span() => {
                let _fut = <#ty as #_form::FromForm<'r>>::push_data(#ctxt, __f.shift());
                _fut.await;
            })))
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn finalize(mut __c: Self::Context) -> #_Result<Self, #_form::Errors<'r>> {
                    #[allow(unused_imports)]
                    use #_form::validate::*;

                    #output
                }
            })
            .try_fields_map(|mapper, fields| {
                // This validates the attributes so we can `unwrap()` later.
                let finalize_field = fields.iter()
                    .map(|f| mapper.map_field(f))
                    .collect::<Result<Vec<TokenStream>>>()?;

                let (_ok, _some, _err, _none) = (_Ok, _Some, _Err, _None);
                let validator = fields.iter().flat_map(|f| validators(f).unwrap());
                let ident = fields.iter().map(|f| f.context_ident());
                let builder = fields.builder(|f| {
                    let ident = f.context_ident();
                    quote!(#ident.unwrap())
                });

                Ok(quote_spanned!(fields.span() =>
                    #(
                        let #ident = match #finalize_field {
                            #_ok(#ident) => #_some(#ident),
                            #_err(__e) => { __c.__errors.extend(__e); #_none }
                        };
                    )*

                    #(
                        if let #_err(__e) = #validator {
                            __c.__errors.extend(__e);
                        }
                    )*

                    if !__c.__errors.is_empty() {
                        return #_Err(__c.__errors);
                    }

                    Ok(#builder)
                ))
            })
            .try_field_map(|_, f| {
                let (ident, ty) = (f.context_ident(), f.stripped_ty());
                let name_buf_opt = f.name_buf_opt()?;
                let default = default(f)?
                    .unwrap_or_else(|| quote_spanned!(ty.span() => {
                        <#ty as #_form::FromForm<'r>>::default(__opts)
                    }));

                let _err = _Err;
                Ok(quote_spanned! { ty.span() => {
                    let __opts = __c.__opts;
                    let __name = #name_buf_opt;
                    __c.#ident
                        .map_or_else(
                            || #default.ok_or_else(|| #_form::ErrorKind::Missing.into()),
                            <#ty as #_form::FromForm<'r>>::finalize
                        )
                        .map_err(|__e| match __name {
                            Some(__name) => __e.with_name(__name),
                            None => __e,
                        })
                        .map_err(|__e| __e.is_empty()
                            .then(|| #_form::ErrorKind::Unknown.into())
                            .unwrap_or(__e))
                }})
            })
        )
        .to_tokens()
}
