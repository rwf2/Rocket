use devise::{*, ext::SpanDiagnosticExt};
use proc_macro2::TokenStream;
use syn::{ConstParam, Index, LifetimeParam, Member, TypeParam};

use crate::exports::{*, Status as _Status};
use crate::http_codegen::Status;

#[derive(Debug, Default, FromMeta)]
struct ItemAttr {
    status: Option<SpanWrapped<Status>>,
    /// Option to generate a respond_to impl with the debug repr of the type
    debug: bool,
}

#[derive(Default, FromMeta)]
struct FieldAttr {
    source: bool,
}

pub fn derive_typed_error(input: proc_macro::TokenStream) -> TokenStream {
    let impl_tokens = quote!(impl<'r> #TypedError<'r>);
    let typed_error: TokenStream = DeriveGenerator::build_for(input.clone(), impl_tokens)
        .support(Support::Struct | Support::Enum | Support::Lifetime | Support::Type)
        .replace_generic(0, 0)
        .type_bound_mapper(MapperBuild::new()
            .input_map(|_, i| {
                let bounds = i.generics().type_params().map(|g| &g.ident);
                quote! { #(#bounds: ::std::marker::Send + ::std::marker::Sync + 'static,)* }
            })
        )
        .validator(ValidatorBuild::new()
            .input_validate(|_, i| match i.generics().lifetimes().count() > 1 {
                true => Err(i.generics().span().error("only one lifetime is supported")),
                false => Ok(())
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                #[allow(unused_variables)]
                fn respond_to(&self, request: &'r #Request<'_>)
                    -> #_Result<#Response<'r>, #_Status>
                {
                    #output
                }
            })
            .try_fields_map(|_, fields| {
                let item = ItemAttr::one_from_attrs("error", fields.parent.attrs())?.unwrap_or(Default::default());
                let status = item.status.map_or(quote!(#_Status::InternalServerError), |m| quote!(#m));
                Ok(if item.debug {
                    quote! {
                        use #_response::Responder;
                        #_response::Debug(self)
                            .respond_to(request)
                            .map_err(|_| #status)
                            .map(|mut r| { r.set_status(#status); r })
                    }
                } else {
                    quote! {
                        #_Err(#status)
                    }
                })
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn source(&'r self) -> #_Option<&'r (dyn #TypedError<'r> + 'r)> {
                    #output
                }
            })
            .try_fields_map(|_, fields| {
                let mut source = None;
                for field in fields.iter() {
                    if FieldAttr::one_from_attrs("error", &field.attrs)?.is_some_and(|a| a.source) {
                        if source.is_some() {
                            return Err(Diagnostic::spanned(
                                field.span(),
                                Level::Error,
                                "Only one field may be declared as `#[error(source)]`"));
                        }
                        if let FieldParent::Variant(_) = field.parent {
                            let name = field.match_ident();
                            source = Some(quote! { #_Some(#name as &dyn #TypedError<'r>) })
                        } else {
                            let span = field.field.span().into();
                            let member = match field.ident {
                                Some(ref ident) => Member::Named(ident.clone()),
                                None => Member::Unnamed(Index { index: field.index as u32, span })
                            };

                            source = Some(quote_spanned!(
                                span => #_Some(&self.#member as &dyn #TypedError<'r>
                            )));
                        }
                    }
                }
                Ok(source.unwrap_or_else(|| quote! { #_None }))
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn status(&self) -> #_Status { #output }
            })
            .try_fields_map(|_, fields| {
                let item = ItemAttr::one_from_attrs("error", fields.parent.attrs())?.unwrap_or(Default::default());
                let status = item.status.map_or(quote!(#_Status::InternalServerError), |m| quote!(#m));
                Ok(quote! { #status })
            })
        )
        .to_tokens();
    let impl_tokens = quote!(unsafe impl #_catcher::Transient);
    let transient: TokenStream = DeriveGenerator::build_for(input, impl_tokens)
        .support(Support::Struct | Support::Enum | Support::Lifetime | Support::Type)
        .replace_generic(1, 0)
        .type_bound_mapper(MapperBuild::new()
            .input_map(|_, i| {
                let bounds = i.generics().type_params().map(|g| &g.ident);
                quote! { #(#bounds: 'static,)* }
            })
        )
        .validator(ValidatorBuild::new()
            .input_validate(|_, i| match i.generics().lifetimes().count() > 1 {
                true => Err(i.generics().span().error("only one lifetime is supported")),
                false => Ok(())
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                #output
            })
            .input_map(|_, input| {
                let name = input.ident();
                let args = input.generics()
                    .params
                    .iter()
                    .map(|g| {
                        match g {
                            syn::GenericParam::Lifetime(_) => quote!{ 'static },
                            syn::GenericParam::Type(TypeParam { ident, .. }) => quote! { #ident },
                            syn::GenericParam::Const(ConstParam { .. }) => todo!(),
                        }
                    });
                let trans = input.generics()
                    .lifetimes()
                    .map(|LifetimeParam { lifetime, .. }| quote!{#_catcher::Inv<#lifetime>});
                quote!{
                    type Static = #name <#(#args)*>;
                    type Transience = (#(#trans,)*);
                }
            })
        )
        // TODO: hack to generate unsafe impl
        .outer_mapper(MapperBuild::new()
            .input_map(|_, _| quote!{ unsafe })
        )
        .to_tokens();
    quote!{
        #typed_error
        #transient
    }
}
