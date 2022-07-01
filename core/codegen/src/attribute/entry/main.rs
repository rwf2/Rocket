use super::EntryAttr;

use devise::{Spanned, Result};
use devise::ext::SpanDiagnosticExt;
use proc_macro2::{TokenStream, Span};

/// `#[rocket::async_main]`: calls the attributed fn inside `rocket::async_main`
pub struct Main;

impl EntryAttr for Main {
    const REQUIRES_ASYNC: bool = true;

    fn function(f: &mut syn::ItemFn) -> Result<TokenStream> {
        let (attrs, vis, block, sig) = (&f.attrs, &f.vis, &f.block, &mut f.sig);
        if sig.ident != "main" {
            // FIXME(diag): warning!
            Span::call_site()
                .warning("attribute is typically applied to `main` function")
                .span_note(sig.ident.span(), "this function is not `main`")
                .emit_as_item_tokens();
        }

        sig.asyncness = None;
        Ok(quote_spanned!(block.span() => #(#attrs)* #vis #sig {
            ::rocket::async_main(async move #block)
        }))
    }
}
