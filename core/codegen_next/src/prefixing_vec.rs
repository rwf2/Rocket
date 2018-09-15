use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use syn::{Ident, Path};

use derive_utils::parser::{Parser, Seperator, Result as PResult};

#[inline]
fn prefix_path(prefix: &str, path: &mut Path) {
    let mut last_seg = path.segments.last_mut().expect("last path segment");
    let last_value = last_seg.value_mut();
    last_value.ident = Ident::new(&format!("{}{}", prefix, last_value.ident), last_value.ident.span());
}

pub fn prefixing_vec_macro_internal<F>(prefix: &str, to_expr: F, args: TokenStream) -> PResult<TokenStream>
        where F: FnMut(Path) -> TokenStream2
{
    let mut parser = Parser::new(args);
    let mut paths = parser.parse_sep(Seperator::Comma, |p| {
        p.parse::<Path>()
    })?;
    parser.eof().map_err(|_| {
        parser.current_span()
            .error("expected `,` or `::` or end of macro invocation")
    })?;

    for ref mut p in &mut paths {
        prefix_path(prefix, p);
    }
    let path_exprs: Vec<_> = paths.into_iter().map(to_expr).collect();

    let tokens = quote! { vec![#(#path_exprs),*] };
    Ok(tokens.into())
}

pub fn prefixing_vec_macro<F>(prefix: &str, to_expr: F, args: TokenStream) -> TokenStream
    where F: FnMut(Path) -> TokenStream2
{
    prefixing_vec_macro_internal(prefix, to_expr, args)
        .unwrap_or_else(|diag| {
            diag.emit();
            (quote! { vec![] }).into()
        })
}
