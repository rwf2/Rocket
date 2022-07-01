use std::hash::Hash;

use devise::Spanned;
use devise::ext::SpanDiagnosticExt;
use proc_macro2::{TokenStream, TokenTree, Punct};

use crate::syn_ext::IdentExt;

pub fn _macro(input: proc_macro::TokenStream) -> devise::Result<TokenStream> {
    let mac: syn::ItemMacro = syn::parse(input)?;
    let macro_name = match mac.ident {
        Some(ident) => ident,
        None => return Err(mac.span().error("expected `macro_rules!`")),
    };

    // We rename the actual `macro_export` macro so we don't accidentally use it
    // internally from the auto-imported crate root macro namespace.
    let (attrs, def) = (mac.attrs, mac.mac);
    let internal_name = macro_name.prepend("___internal_");
    let mod_name = macro_name.uniqueify_with(|mut hasher| def.hash(&mut hasher));

    let macro_rules_tokens = def.tokens.clone();
    let decl_macro_tokens: TokenStream = def.tokens.into_iter()
        .map(|t| match t {
            TokenTree::Punct(p) if p.as_char() == ';' => {
                let mut token = Punct::new(',', p.spacing());
                token.set_span(p.span());
                TokenTree::Punct(token)
            },
            _ => t,
        })
        .collect();

    Ok(quote! {
        #[allow(non_snake_case)]
        mod #mod_name {
            #[doc(hidden)]
            #[macro_export]
            macro_rules! #internal_name {
                #macro_rules_tokens
            }

            pub use #internal_name;
        }

        #(#attrs)*
        #[cfg(all(nightly, doc))]
        pub macro #macro_name {
            #decl_macro_tokens
        }

        #[cfg(not(all(nightly, doc)))]
        pub use #mod_name::#internal_name as #macro_name;
    })
}
