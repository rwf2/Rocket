#![feature(proc_macro, core_intrinsics, decl_macro)]
#![recursion_limit="256"]

extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate quote;

mod parser;
mod spanned;
mod ext;
mod codegen_ext;

use parser::Result as PResult;
use proc_macro::{Span, TokenStream};
use spanned::Spanned;

use ext::*;
use syn::*;

const NO_FIELDS_ERR: &str = "variants in `FromFormValue` derives cannot have fields";
const NO_GENERICS: &str = "enums with generics cannot derive `FromFormValue`";
const ONLY_ENUMS: &str = "`FromFormValue` can only be derived for enums";
const EMPTY_ENUM_WARN: &str = "deriving `FromFormValue` for empty enum";

#[derive(Debug, Clone)]
pub(crate) struct FieldMember<'f> {
    field: &'f Field,
    member: Member
}

fn validate_input(input: DeriveInput) -> PResult<DataEnum> {
    // This derive doesn't support generics. Error out if there are generics.
    if !input.generics.params.is_empty() {
        return Err(input.generics.span().error(NO_GENERICS));
    }

    // This derive only works for enums. Error out if the input is not an enum.
    let input_span = input.span();
    let data = input.data.into_enum().ok_or_else(|| input_span.error(ONLY_ENUMS))?;

    // This derive only works for variants that are nullary.
    for variant in data.variants.iter() {
        if !variant.fields.is_empty() {
            return Err(variant.span().error(NO_FIELDS_ERR));
        }
    }

    // Emit a warning if the enum is empty.
    if data.variants.is_empty() {
        Span::call_site().warning(EMPTY_ENUM_WARN).emit();
    }

    Ok(data)
}

fn real_derive_from_form_value(input: TokenStream) -> PResult<TokenStream> {
    // Parse the input `TokenStream` as a `syn::DeriveInput`, an AST.
    let input: DeriveInput = syn::parse(input).map_err(|e| {
        Span::call_site().error(format!("error: failed to parse input: {:?}", e))
    })?;

    // Validate the enum.
    let name = input.ident.clone();
    let enum_data = validate_input(input)?;

    // Create iterators over the identifers as idents and as strings.
    let variant_strs = enum_data.variants.iter().map(|v| v.ident.to_string());
    let variant_idents = enum_data.variants.iter().map(|v| v.ident.clone());
    let names = ::std::iter::repeat(&name);

    // Generate the implementation.
    Ok(quote! {
        mod scope {
            extern crate std;
            extern crate rocket;

            use self::std::prelude::v1::*;
            use self::rocket::request::FromFormValue;
            use self::rocket::http::RawStr;

            impl<'v> FromFormValue<'v> for #name {
                type Error = &'v RawStr;

                fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
                    #(if v.as_uncased_str() == #variant_strs {
                        return Ok(#names::#variant_idents);
                    })*

                    Err(v)
                }
            }
        }
    }.into())
}

#[proc_macro_derive(FromFormValue)]
pub fn derive_from_form_value(input: TokenStream) -> TokenStream {
    real_derive_from_form_value(input).unwrap_or_else(|diag| {
        diag.emit();
        TokenStream::empty()
    })
}
