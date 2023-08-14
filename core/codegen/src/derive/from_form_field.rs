use devise::{*, ext::SpanDiagnosticExt};
use proc_macro2::TokenStream;

use crate::exports::*;
use crate::derive::form_field::{VariantExt, first_duplicate};

pub fn derive_from_form_field(input: proc_macro::TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl<'__v> #_form::FromFormField<'__v>))
        .support(Support::Enum)
        .validator(ValidatorBuild::new()
            // We only accept C-like enums with at least one variant.
            .fields_validate(|_, fields| {
                if !fields.is_empty() {
                    return Err(fields.span().error("variants cannot have fields"));
                }

                Ok(())
            })
            .enum_validate(|_, data| {
                if data.variants.is_empty() {
                    return Err(data.span().error("enum must have at least one variant"));
                }

                if let Some(d) = first_duplicate(data.variants(), |v| v.form_field_values())? {
                    let (variant_a_i, variant_a, value_a) = d.0;
                    let (variant_b_i, variant_b, value_b) = d.1;

                    if variant_a_i == variant_b_i {
                        return Err(variant_a.error("variant has conflicting values")
                            .span_note(value_a, "this value...")
                            .span_note(value_b, "...conflicts with this value"));
                    }

                    return Err(value_b.error("field value conflicts with previous value")
                        .span_help(variant_b, "...declared in this variant")
                        .span_note(variant_a, "previous field with conflicting name"));
                }

                Ok(())
            })
        )
        .outer_mapper(quote! {
            #[allow(unused_imports)]
            use #_http::uncased::AsUncased;
        })
        .inner_mapper(MapperBuild::new()
            .with_output(|_, output| quote! {
                fn from_value(
                    __f: #_form::ValueField<'__v>
                ) -> #_Result<Self, #_form::Errors<'__v>> {

                    #output
                }
            })
            .try_enum_map(|mapper, data| {
                let mut variant_value = vec![];
                for v in data.variants().map(|v| v.form_field_values()) {
                    variant_value.append(&mut v?);
                }

                let variant_condition = data.variants()
                    .map(|v| mapper.map_variant(v))
                    .collect::<Result<Vec<_>>>()?;

                let (_ok, _cow) = (std::iter::repeat(_Ok), std::iter::repeat(_Cow));
                Ok(quote! {
                    #(#variant_condition)*

                    const OPTS: &'static [#_Cow<'static, str>] =
                        &[#(#_cow::Borrowed(#variant_value)),*];

                    let _error = #_form::Error::from(OPTS)
                        .with_name(__f.name)
                        .with_value(__f.value);

                    #_Err(_error)?
                })
            })
            .try_variant_map(|_, variant| {
                let builder = variant.builder(|_| unreachable!("fieldless"));
                let value = variant.form_field_values()?;

                Ok(quote_spanned! { variant.span() =>
                    if #(__f.value.as_uncased() == #value)||* {
                        return #_Ok(#builder);
                    }
                })
            })
        )
        .to_tokens()
}
