use super::EntryAttr;

use devise::{Spanned, Result};
use proc_macro2::TokenStream;

/// `#[rocket::async_test]`: calls the attributed fn inside `rocket::async_test`
pub struct Test;

impl EntryAttr for Test {
    const REQUIRES_ASYNC: bool = true;

    fn function(f: &mut syn::ItemFn) -> Result<TokenStream> {
        let (attrs, vis, block, sig) = (&f.attrs, &f.vis, &f.block, &mut f.sig);
        sig.asyncness = None;
        Ok(quote_spanned!(block.span() => #(#attrs)* #[test] #vis #sig {
            ::rocket::async_test(async move #block)
        }))
    }
}
