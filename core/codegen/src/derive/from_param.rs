use devise::Support;
use proc_macro2::TokenStream;
use quote::quote;
use devise::*;
use devise::ext::SpanDiagnosticExt;
use crate::exports::*;

pub fn derive_from_param(input: proc_macro::TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl<'a> #_request::FromParam<'a>))
        .support(Support::Enum)
        .validator(ValidatorBuild::new()
                   .fields_validate(|_, fields| {
                       if !fields.is_empty() {
                           return Err(
                               fields.span().error("Only enums without data fields are supported")
                           );
                       }
                       Ok(())
                   })
        )
        .inner_mapper(MapperBuild::new()
                      .enum_map(|_, data| {
                          let matches = data.variants().map(|field| {
                              let field_name = &field;
                              quote!(
                                  stringify!(#field_name) => Ok(Self::#field_name),
                              )

                          });

                          quote! {
                              type Error = &'a str;
                              fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                                  match param {
                                      #(#matches)*
                                      _ => Err("Failed to find enum")
                                  }
                              }
                          }
                      })
        )
        .to_tokens()
}
