use proc_macro2::TokenStream;
use devise::{*, ext::SpanDiagnosticExt};

use crate::exports::*;
use crate::derive::form_field::{FieldExt, VariantExt};
use crate::syn_ext::{GenericsExt as _, TypeExt as _};
use crate::http::uri::fmt;

const NO_EMPTY_FIELDS: &str = "fieldless structs are not supported";
const NO_NULLARY: &str = "nullary items are not supported";
const NO_EMPTY_ENUMS: &str = "empty enums are not supported";
const ONLY_ONE_UNNAMED: &str = "tuple structs or variants must have exactly one field";
const EXACTLY_ONE_FIELD: &str = "struct must have exactly one field";

const Q_URI_DISPLAY: StaticTokens = quote_static!(#_fmt::UriDisplay<#_fmt::Query>);
const Q_FORMATTER: StaticTokens = quote_static!(#_fmt::Formatter<#_fmt::Query>);

const P_URI_DISPLAY: StaticTokens = quote_static!(#_fmt::UriDisplay<#_fmt::Path>);
const P_FORMATTER: StaticTokens = quote_static!(#_fmt::Formatter<#_fmt::Path>);

fn generic_bounds_mapper(bound: StaticTokens) -> MapperBuild {
    MapperBuild::new()
        .try_enum_map(|m, e| mapper::enum_null(m, e))
        .try_fields_map(move |_, fields| {
            let generic_idents = fields.parent.input().generics().type_idents();

            let bounds = fields.iter()
                .filter(|f| !f.ty.is_concrete(&generic_idents))
                .map(|f| &f.field.inner.ty)
                .map(move |ty| quote_spanned!(ty.span() => #ty: #bound));

            Ok(quote!(#(#bounds,)*))
        })
}

pub fn derive_uri_display_query(input: proc_macro::TokenStream) -> TokenStream {
    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #Q_URI_DISPLAY))
        .support(Support::Struct | Support::Enum | Support::Type | Support::Lifetime)
        .validator(ValidatorBuild::new()
            .enum_validate(|_, data| {
                if data.variants().count() == 0 {
                    Err(data.brace_token.span.error(NO_EMPTY_ENUMS))
                } else {
                    Ok(())
                }
            })
            .struct_validate(|_, data| {
                let fields = data.fields();
                if fields.is_empty() {
                    Err(data.span().error(NO_EMPTY_FIELDS))
                } else if fields.are_unit() {
                    Err(data.span().error(NO_NULLARY))
                } else {
                    Ok(())
                }
            })
            .fields_validate(|_, fields| {
                if fields.are_unnamed() && fields.count() > 1 {
                    Err(fields.span().error(ONLY_ONE_UNNAMED))
                } else {
                    Ok(())
                }
            })
        )
        .type_bound_mapper(generic_bounds_mapper(Q_URI_DISPLAY))
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn fmt(&self, f: &mut #Q_FORMATTER) -> ::std::fmt::Result {
                    #output
                    Ok(())
                }
            })
            .try_variant_map(|mapper, variant| {
                if !variant.fields().is_empty() {
                    return mapper::variant_default(mapper, variant);
                }

                let value = variant.first_form_field_value()?;
                Ok(quote_spanned! { variant.span() =>
                    f.write_value(#value)?;
                })
            })
            .try_field_map(|_, field| {
                let span = field.span();
                let accessor = field.accessor();
                let tokens = if let Some(name) = field.first_field_name()? {
                    quote_spanned!(span => f.write_named_value(#name, &#accessor)?;)
                } else {
                    quote_spanned!(span => f.write_value(&#accessor)?;)
                };

                Ok(tokens)
            })
        )
        .try_to_tokens::<TokenStream>();

    let uri_display = match uri_display {
        Ok(tokens) => tokens,
        Err(diag) => return diag.emit_as_item_tokens()
    };

    let from_self = from_uri_param::<fmt::Query>(input.clone(), quote!(Self));
    let from_ref = from_uri_param::<fmt::Query>(input.clone(), quote!(&'__r Self));
    let from_mut = from_uri_param::<fmt::Query>(input, quote!(&'__r mut Self));

    let mut ts = uri_display;
    ts.extend(from_self);
    ts.extend(from_ref);
    ts.extend(from_mut);
    ts
}

#[allow(non_snake_case)]
pub fn derive_uri_display_path(input: proc_macro::TokenStream) -> TokenStream {
    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #P_URI_DISPLAY))
        .support(Support::TupleStruct | Support::Type | Support::Lifetime)
        .type_bound_mapper(generic_bounds_mapper(P_URI_DISPLAY))
        .validator(ValidatorBuild::new()
            .fields_validate(|_, fields| match fields.count() {
                1 => Ok(()),
                _ => Err(fields.span().error(EXACTLY_ONE_FIELD))
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn fmt(&self, f: &mut #P_FORMATTER) -> ::std::fmt::Result {
                    #output
                    Ok(())
                }
            })
            .field_map(|_, field| {
                let accessor = field.accessor();
                quote_spanned!(field.span() => f.write_value(&#accessor)?;)
            })
        )
        .try_to_tokens::<TokenStream>();

    let uri_display = match uri_display {
        Ok(tokens) => tokens,
        Err(diag) => return diag.emit_as_item_tokens()
    };

    let from_self = from_uri_param::<fmt::Path>(input.clone(), quote!(Self));
    let from_ref = from_uri_param::<fmt::Path>(input.clone(), quote!(&'__r Self));
    let from_mut = from_uri_param::<fmt::Path>(input, quote!(&'__r mut Self));

    let mut ts = uri_display;
    ts.extend(from_self);
    ts.extend(from_ref);
    ts.extend(from_mut);
    ts
}

fn from_uri_param<P: fmt::Part>(input: proc_macro::TokenStream, ty: TokenStream) -> TokenStream {
    let part = match P::KIND {
        fmt::Kind::Path => quote!(#_fmt::Path),
        fmt::Kind::Query => quote!(#_fmt::Query),
    };

    let display_trait = match P::KIND {
        fmt::Kind::Path => P_URI_DISPLAY,
        fmt::Kind::Query => Q_URI_DISPLAY,
    };

    let ty: syn::Type = syn::parse2(ty).expect("valid type");
    let gen = match ty {
        syn::Type::Reference(ref r) => r.lifetime.as_ref().map(|l| quote!(<#l>)),
        _ => None
    };

    let param_trait = quote!(impl #gen #_fmt::FromUriParam<#part, #ty>);
    DeriveGenerator::build_for(input, param_trait)
        .support(Support::All)
        .type_bound_mapper(generic_bounds_mapper(display_trait))
        .inner_mapper(MapperBuild::new()
            .with_output(move |_, _| quote! {
                type Target = #ty;
                #[inline(always)] fn from_uri_param(_p: #ty) -> #ty { _p }
            })
        )
        .to_tokens()
}
