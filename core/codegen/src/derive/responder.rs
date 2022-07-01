use quote::ToTokens;
use devise::{*, ext::{TypeExt, SpanDiagnosticExt}};
use proc_macro2::TokenStream;

use crate::exports::*;
use crate::syn_ext::{TypeExt as _, GenericsExt as _};
use crate::http_codegen::{ContentType, Status};

#[derive(Debug, Default, FromMeta)]
struct ItemAttr {
    content_type: Option<SpanWrapped<ContentType>>,
    status: Option<SpanWrapped<Status>>,
}

#[derive(Default, FromMeta)]
struct FieldAttr {
    ignore: bool,
}

pub fn derive_responder(input: proc_macro::TokenStream) -> TokenStream {
    let impl_tokens = quote!(impl<'r, 'o: 'r> #_response::Responder<'r, 'o>);
    DeriveGenerator::build_for(input, impl_tokens)
        .support(Support::Struct | Support::Enum | Support::Lifetime | Support::Type)
        .replace_generic(1, 0)
        .type_bound_mapper(MapperBuild::new()
            .try_enum_map(|m, e| mapper::enum_null(m, e))
            .try_fields_map(|_, fields| {
                let generic_idents = fields.parent.input().generics().type_idents();
                let lifetime = |ty: &syn::Type| syn::Lifetime::new("'o", ty.span());
                let mut types = fields.iter()
                    .map(|f| (f, &f.field.inner.ty))
                    .map(|(f, ty)| (f, ty.with_replaced_lifetimes(lifetime(ty))));

                let mut bounds = vec![];
                if let Some((_, ty)) = types.next() {
                    if !ty.is_concrete(&generic_idents) {
                        let span = ty.span();
                        bounds.push(quote_spanned!(span => #ty: #_response::Responder<'r, 'o>));
                    }
                }

                for (f, ty) in types {
                    let attr = FieldAttr::one_from_attrs("response", &f.attrs)?.unwrap_or_default();
                    if ty.is_concrete(&generic_idents) || attr.ignore {
                        continue;
                    }

                    bounds.push(quote_spanned! { ty.span() =>
                        #ty: ::std::convert::Into<#_http::Header<'o>>
                    });
                }

                Ok(quote!(#(#bounds,)*))
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
                    quote_spanned! { f.span().into() =>
                        let mut __res = <#ty as #_response::Responder>::respond_to(
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
