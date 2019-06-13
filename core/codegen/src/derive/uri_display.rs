use proc_macro::{Span, TokenStream};
use devise::*;

use crate::derive::from_form::Form;
use crate::proc_macro2::TokenStream as TokenStream2;

const NO_EMPTY_FIELDS: &str = "fieldless structs or variants are not supported";
const NO_NULLARY: &str = "nullary items are not supported";
const NO_EMPTY_ENUMS: &str = "empty enums are not supported";
const ONLY_ONE_UNNAMED: &str = "tuple structs or variants must have exactly one field";
const EXACTLY_ONE_FIELD: &str = "struct must have exactly one field";

fn validate_fields(fields: Fields<'_>, parent_span: Span) -> Result<()> {
    if fields.count() == 0 {
        return Err(parent_span.error(NO_EMPTY_FIELDS))
    } else if fields.are_unnamed() && fields.count() > 1 {
        return Err(fields.span().error(ONLY_ONE_UNNAMED));
    } else if fields.are_unit() {
        return Err(parent_span.error(NO_NULLARY));
    }

    Ok(())
}

fn validate_struct(gen: &DeriveGenerator, data: Struct<'_>) -> Result<()> {
    validate_fields(data.fields(), gen.input.span())
}

fn validate_enum(gen: &DeriveGenerator, data: Enum<'_>) -> Result<()> {
    if data.variants().count() == 0 {
        return Err(gen.input.span().error(NO_EMPTY_ENUMS));
    }

    for variant in data.variants() {
        validate_fields(variant.fields(), variant.span())?;
    }

    Ok(())
}

#[allow(non_snake_case)]
pub fn derive_uri_display_query(input: TokenStream) -> TokenStream {
    let Query = quote!(::rocket::http::uri::Query);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Query>);
    let Formatter = quote!(::rocket::http::uri::Formatter<#Query>);
    let FromUriParam = quote!(::rocket::http::uri::FromUriParam);

    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #UriDisplay))
        .data_support(DataSupport::Struct | DataSupport::Enum)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .validate_enum(validate_enum)
        .validate_struct(validate_struct)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(move |_, inner| quote! {
            fn fmt(&self, f: &mut #Formatter) -> ::std::fmt::Result {
                #inner
                Ok(())
            }
        })
        .try_map_field(|_, field| {
            let span = field.span().into();
            let accessor = field.accessor();
            let tokens = if let Some(ref ident) = field.ident {
                let name = Form::from_attrs("form", &field.attrs)
                    .map(|result| result.map(|form| form.field.name))
                    .unwrap_or_else(|| Ok(ident.to_string()))?;

                quote_spanned!(span => f.write_named_value(#name, &#accessor)?;)
            } else {
                quote_spanned!(span => f.write_value(&#accessor)?;)
            };

            Ok(tokens)
        })
        .to_tokens();

    let i = input.clone();
    let gen_trait = quote!(impl #FromUriParam<#Query, Self>);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Query>);
    let from_self = DeriveGenerator::build_for(i, gen_trait)
        .data_support(DataSupport::Struct | DataSupport::Enum)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(|_, _| quote! {
            type Target = Self;
            #[inline(always)]
            fn from_uri_param(param: Self) -> Self { param }
        })
        .to_tokens();

    let i = input.clone();
    let gen_trait = quote!(impl<'__r> #FromUriParam<#Query, &'__r Self>);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Query>);
    let from_ref = DeriveGenerator::build_for(i, gen_trait)
        .data_support(DataSupport::Struct | DataSupport::Enum)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(|_, _| quote! {
            type Target = &'__r Self;
            #[inline(always)]
            fn from_uri_param(param: &'__r Self) -> &'__r Self { param }
        })
        .to_tokens();

    let i = input.clone();
    let gen_trait = quote!(impl<'__r> #FromUriParam<#Query, &'__r mut Self>);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Query>);
    let from_mut = DeriveGenerator::build_for(i, gen_trait)
        .data_support(DataSupport::Struct | DataSupport::Enum)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(|_, _| quote! {
            type Target = &'__r mut Self;
            #[inline(always)]
            fn from_uri_param(param: &'__r mut Self) -> &'__r mut Self { param }
        })
        .to_tokens();

    let mut ts = TokenStream2::from(uri_display);
    ts.extend(TokenStream2::from(from_self));
    ts.extend(TokenStream2::from(from_ref));
    ts.extend(TokenStream2::from(from_mut));
    ts.into()
}

#[allow(non_snake_case)]
pub fn derive_uri_display_path(input: TokenStream) -> TokenStream {
    let Path = quote!(::rocket::http::uri::Path);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Path>);
    let Formatter = quote!(::rocket::http::uri::Formatter<#Path>);
    let FromUriParam = quote!(::rocket::http::uri::FromUriParam);

    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #UriDisplay))
        .data_support(DataSupport::TupleStruct)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .validate_fields(|_, fields| match fields.count() {
            1 => Ok(()),
            _ => Err(fields.span().error(EXACTLY_ONE_FIELD))
        })
        .function(move |_, inner| quote! {
            fn fmt(&self, f: &mut #Formatter) -> ::std::fmt::Result {
                #inner
                Ok(())
            }
        })
        .map_field(|_, field| {
            let span = field.span().into();
            let accessor = field.accessor();
            quote_spanned!(span => f.write_value(&#accessor)?;)
        })
        .to_tokens();

    let i = input.clone();
    let gen_trait = quote!(impl #FromUriParam<#Path, Self>);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Path>);
    let from_self = DeriveGenerator::build_for(i, gen_trait)
        .data_support(DataSupport::All)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(|_, _| quote! {
            type Target = Self;
            #[inline(always)]
            fn from_uri_param(param: Self) -> Self { param }
        })
        .to_tokens();

    let i = input.clone();
    let gen_trait = quote!(impl<'__r> #FromUriParam<#Path, &'__r Self>);
    let UriDisplay = quote!(::rocket::http::uri::UriDisplay<#Path>);
    let from_ref = DeriveGenerator::build_for(i, gen_trait)
        .data_support(DataSupport::All)
        .generic_support(GenericSupport::Type | GenericSupport::Lifetime)
        .map_type_generic(move |_, ident, _| quote!(#ident : #UriDisplay))
        .function(|_, _| quote! {
            type Target = &'__r Self;
            #[inline(always)]
            fn from_uri_param(param: &'__r Self) -> &'__r Self { param }
        })
        .to_tokens();

    let mut ts = TokenStream2::from(uri_display);
    ts.extend(TokenStream2::from(from_self));
    ts.extend(TokenStream2::from(from_ref));
    ts.into()
}
