use devise::{*, ext::SpanDiagnosticExt};

use crate::proc_macro2::TokenStream;

#[derive(FromMeta)]
struct Form {
    value: String,
}

pub fn derive_from_form_value(input: proc_macro::TokenStream) -> TokenStream {
    define_vars_and_mods!(_Ok, _Err, _Result);
    DeriveGenerator::build_for(input, quote!(impl<'__v> ::rocket::request::FromFormValue<'__v>))
        .generic_support(GenericSupport::None)
        .data_support(DataSupport::Enum)
        .validate_enum(|_, data| {
            // This derive only works for variants that are nullary.
            for variant in data.variants() {
                if !variant.fields().is_empty() {
                    return Err(variant.fields().span().error("variants cannot have fields"));
                }
            }

            // Emit a warning if the enum is empty.
            if data.variants.is_empty() {
                return Err(data.brace_token.span.error("enum must have at least one field"));
            }

            Ok(())
        })
        .function(move |_, inner| quote! {
            type Error = &'__v ::rocket::http::RawStr;

            fn from_form_value(
                value: &'__v ::rocket::http::RawStr
            ) -> #_Result<Self, Self::Error> {
                let uncased = value.as_uncased_str();
                #inner
                #_Err(value)
            }
        })
        .try_map_enum(null_enum_mapper)
        .try_map_variant(|_, variant| {
            define_vars_and_mods!(_Ok);
            let variant_str = Form::from_attrs("form", &variant.attrs)
                .unwrap_or_else(|| Ok(Form { value: variant.ident.to_string() }))?
                .value;

            let builder = variant.builder(|_| unreachable!());
            Ok(quote! {
                if uncased == #variant_str {
                    return #_Ok(#builder);
                }
            })
        })
        .to_tokens2()
}
