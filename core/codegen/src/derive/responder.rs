use quote::ToTokens;
use devise::{*, ext::{TypeExt, SpanDiagnosticExt, GenericsExt}};
use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::parse::Parser;

use crate::exports::*;
use crate::http_codegen::{ContentType, Status};

type WherePredicates = Punctuated<syn::WherePredicate, syn::Token![,]>;

#[derive(Debug, Default, FromMeta)]
struct ItemAttr {
    bound: Option<SpanWrapped<String>>,
    content_type: Option<SpanWrapped<ContentType>>,
    status: Option<SpanWrapped<Status>>,
}

#[derive(Default, FromMeta)]
struct FieldAttr {
    ignore: bool,
}

pub fn derive_responder(input: proc_macro::TokenStream) -> TokenStream {
    let impl_tokens = quote!(impl<'r, 'o: 'r> ::rocket::response::Responder<'r, 'o>);
    DeriveGenerator::build_for(input, impl_tokens)
        .support(Support::Struct | Support::Enum | Support::Lifetime | Support::Type)
        .replace_generic(1, 0)
        .type_bound_mapper(MapperBuild::new()
            .try_input_map(|_, input| {
                ItemAttr::one_from_attrs("response", input.attrs())?
                    .and_then(|attr| attr.bound)
                    .map(|bound| {
                        let span = bound.span;
                        let bounds = WherePredicates::parse_terminated.parse_str(&bound)
                            .map_err(|e| span.error(format!("invalid bound syntax: {}", e)))?;
                        Ok(quote_respanned!(span => #bounds))
                    })
                    .unwrap_or_else(|| {
                        let bound = quote!(::rocket::response::Responder<'r, 'o>);
                        let preds = input.generics().parsed_bounded_types(bound)?;
                        Ok(quote!(#preds))
                    })
            })
        )
        .validator(ValidatorBuild::new()
            .input_validate(|_, i| match i.generics().lifetimes().count() > 1 {
                true => Err(i.generics().span().error("only one lifetime is supported")),
                false => Ok(())
            })
            .fields_validate(|_, fields| match fields.is_empty() {
                true => Err(fields.span().error("need at least one field")),
                false => Ok(())
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn respond_to(self, __req: &'r #Request<'_>) -> #_response::Result<'o> {
                    #output
                }
            })
            .try_fields_map(|_, fields| {
                fn set_header_tokens<T: ToTokens + Spanned>(item: T) -> TokenStream {
                    quote_spanned!(item.span() => __res.set_header(#item);)
                }

                let attr = ItemAttr::one_from_attrs("response", fields.parent.attrs())?
                    .unwrap_or_default();

                let responder = fields.iter().next().map(|f| {
                    let (accessor, ty) = (f.accessor(), f.ty.with_stripped_lifetimes());
                    quote_spanned! { f.span() =>
                        let mut __res = <#ty as ::rocket::response::Responder>::respond_to(
                            #accessor, __req
                        )?;
                    }
                }).expect("have at least one field");

                let mut headers = vec![];
                for field in fields.iter().skip(1) {
                    let attr = FieldAttr::one_from_attrs("response", &field.attrs)?
                        .unwrap_or_default();

                    if !attr.ignore {
                        headers.push(set_header_tokens(field.accessor()));
                    }
                }

                let content_type = attr.content_type.map(set_header_tokens);
                let status = attr.status.map(|status| {
                    quote_spanned!(status.span() => __res.set_status(#status);)
                });

                Ok(quote! {
                    #responder
                    #(#headers)*
                    #content_type
                    #status
                    #_Ok(__res)
                })
            })
        )
        .to_tokens()
}
