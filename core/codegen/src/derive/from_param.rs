use devise::*;
use devise::ext::SpanDiagnosticExt;

use quote::quote;
use proc_macro2::TokenStream;
use syn::ext::IdentExt;

use crate::exports::*;

pub fn derive_from_param(input: proc_macro::TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl<'a> #_request::FromParam<'a>))
        .support(Support::Enum)
        .validator(ValidatorBuild::new().fields_validate(|_, fields| {
            if !fields.is_empty() {
                return Err(fields.span().error("variants with data fields are not supported"));
            }

            Ok(())
        }))
        .inner_mapper(MapperBuild::new().enum_map(|_, data| {
            let matches = data.variants().map(|field| {
                let field_name = field.ident.unraw();
                quote!(stringify!(#field_name) => Ok(Self::#field))
            });

            let names = data.variants().map(|field| {
                let field_name = field.ident.unraw();
                quote!(stringify!(#field_name))
            });

            quote! {
                type Error = #_error::InvalidOption<'a>;

                fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                    match param {
                        #(#matches,)*
                        _ => Err(#_error::InvalidOption::new(param, &[#(#names),*])),
                    }
                }
            }
        }))
        .to_tokens()
}
