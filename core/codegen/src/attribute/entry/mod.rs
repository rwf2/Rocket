mod main;
mod launch;
mod test;

use devise::{Diagnostic, Spanned, Result};
use devise::ext::SpanDiagnosticExt;
use proc_macro2::{TokenStream, Span};

// Common trait implemented by `async` entry generating attributes.
trait EntryAttr {
    /// Whether the attribute requires the attributed function to be `async`.
    const REQUIRES_ASYNC: bool;

    /// Return a new or rewritten function, using block as the main execution.
    fn function(f: &mut syn::ItemFn) -> Result<TokenStream>;
}

fn _async_entry<A: EntryAttr>(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream
) -> Result<TokenStream> {
    let mut function: syn::ItemFn = syn::parse(input)
        .map_err(Diagnostic::from)
        .map_err(|d| d.help("attribute can only be applied to functions"))?;

    if A::REQUIRES_ASYNC && function.sig.asyncness.is_none() {
        return Err(Span::call_site()
            .error("attribute can only be applied to `async` functions")
            .span_note(function.sig.span(), "this function must be `async`"));
    }

    if !function.sig.inputs.is_empty() {
        return Err(Span::call_site()
            .error("attribute can only be applied to functions without arguments")
            .span_note(function.sig.span(), "this function must take no arguments"));
    }

    A::function(&mut function)
}

macro_rules! async_entry {
    ($name:ident, $kind:ty, $default:expr) => (
        pub fn $name(a: proc_macro::TokenStream, i: proc_macro::TokenStream) -> TokenStream {
            _async_entry::<$kind>(a, i).unwrap_or_else(|d| {
                let d = d.emit_as_item_tokens();
                let default = $default;
                quote!(#d #default)
            })
        }
    )
}

async_entry!(async_test_attribute, test::Test, quote!());
async_entry!(main_attribute, main::Main, quote!(fn main() {}));
async_entry!(launch_attribute, launch::Launch, quote!(fn main() {}));
