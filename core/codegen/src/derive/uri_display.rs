use proc_macro2::TokenStream;
use devise::{*, ext::SpanDiagnosticExt};

use crate::exports::*;
use crate::derive::form_field::{FieldExt, VariantExt};
use crate::http::uri::fmt;

const NO_EMPTY_FIELDS: &str = "fieldless structs are not supported";
const NO_NULLARY: &str = "nullary items are not supported";
const NO_EMPTY_ENUMS: &str = "empty enums are not supported";
const ONLY_ONE_UNNAMED: &str = "tuple structs or variants must have exactly one field";
const EXACTLY_ONE_FIELD: &str = "struct must have exactly one field";

pub fn derive_uri_display_query(input: proc_macro::TokenStream) -> TokenStream {
    const URI_DISPLAY: StaticTokens = quote_static!(#_fmt::UriDisplay<#_fmt::Query>);
    const FORMATTER: StaticTokens = quote_static!(#_fmt::Formatter<#_fmt::Query>);

    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #URI_DISPLAY))
        .support(Support::Struct | Support::Enum | Support::Type | Support::Lifetime)
        .validator(ValidatorBuild::new()
            .enum_validate(|_, data| {
                if data.variants().count() == 0 {
                    return Err(data.brace_token.span.error(NO_EMPTY_ENUMS));
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
        .type_bound(URI_DISPLAY)
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn fmt(&self, f: &mut #FORMATTER) -> ::std::fmt::Result {
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
                let span = field.span().into();
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
    let from_mut = from_uri_param::<fmt::Query>(input.clone(), quote!(&'__r mut Self));

    let mut ts = TokenStream::from(uri_display);
    ts.extend(TokenStream::from(from_self));
    ts.extend(TokenStream::from(from_ref));
    ts.extend(TokenStream::from(from_mut));
    ts.into()
}

#[allow(non_snake_case)]
pub fn derive_uri_display_path(input: proc_macro::TokenStream) -> TokenStream {
    const URI_DISPLAY: StaticTokens = quote_static!(#_fmt::UriDisplay<#_fmt::Path>);
    const FORMATTER: StaticTokens = quote_static!(#_fmt::Formatter<#_fmt::Path>);

    let uri_display = DeriveGenerator::build_for(input.clone(), quote!(impl #URI_DISPLAY))
        .support(Support::TupleStruct | Support::Type | Support::Lifetime)
        .type_bound(URI_DISPLAY)
        .validator(ValidatorBuild::new()
            .fields_validate(|_, fields| match fields.count() {
                1 => Ok(()),
                _ => Err(fields.span().error(EXACTLY_ONE_FIELD))
            })
        )
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn fmt(&self, f: &mut #FORMATTER) -> ::std::fmt::Result {
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
    let from_mut = from_uri_param::<fmt::Path>(input.clone(), quote!(&'__r mut Self));

    let mut ts = TokenStream::from(uri_display);
    ts.extend(TokenStream::from(from_self));
    ts.extend(TokenStream::from(from_ref));
    ts.extend(TokenStream::from(from_mut));
    ts.into()
}

fn from_uri_param<P: fmt::Part>(input: proc_macro::TokenStream, ty: TokenStream) -> TokenStream {
    let part = match P::KIND {
        fmt::Kind::Path => quote!(#_fmt::Path),
        fmt::Kind::Query => quote!(#_fmt::Query),
    };

    let ty: syn::Type = syn::parse2(ty).expect("valid type");
    let gen = match ty {
        syn::Type::Reference(ref r) => r.lifetime.as_ref().map(|l| quote!(<#l>)),
        _ => None
    };

    let param_trait = quote!(impl #gen #_fmt::FromUriParam<#part, #ty>);
    DeriveGenerator::build_for(input, param_trait)
        .support(Support::All)
        .type_bound(quote!(#_fmt::UriDisplay<#part>))
        .inner_mapper(MapperBuild::new()
            .with_output(move |_, _| quote! {
                type Target = #ty;
                #[inline(always)] fn from_uri_param(_p: #ty) -> #ty { _p }
            })
        )
        .to_tokens()
}
