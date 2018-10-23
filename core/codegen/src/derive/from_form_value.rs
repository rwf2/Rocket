use derive_utils::*;
use proc_macro::TokenStream;

#[derive(FromMeta)]
struct Form {
    value: String,
}

pub fn derive_from_form_value(input: TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, "::rocket::request::FromFormValue<'__v>")
        .generic_support(GenericSupport::None)
        .data_support(DataSupport::Enum)
        .validate_enum(|generator, data| {
            // This derive only works for variants that are nullary.
            for variant in data.variants() {
                if !variant.fields().is_empty() {
                    return Err(variant.span().error("variants cannot have fields"));
                }
            }

            // Emit a warning if the enum is empty.
            if data.variants.is_empty() {
                generator.input.span().warning("deriving for empty enum").emit();
            }

            Ok(())
        })
        .function(|_, inner| quote! {
            type Error = &'__v ::rocket::http::RawStr;

            fn from_form_value(
                value: &'__v ::rocket::http::RawStr
            ) -> ::std::result::Result<Self, Self::Error> {
                let uncased = value.as_uncased_str();
                #inner
                ::std::result::Result::Err(value)
            }
        })
        .try_map_enum(null_enum_mapper)
        .try_map_variant(|_, variant| {
            let variant_str = Form::from_attrs("form", &variant.attrs)
                .unwrap_or_else(|| Ok(Form { value: variant.ident.to_string() }))?
                .value;

            let builder = variant.builder(|_| unreachable!());
            Ok(quote! {
                if uncased == #variant_str {
                    return ::std::result::Result::Ok(#builder);
                }
            })
        })
        .to_tokens()
}
