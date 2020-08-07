use std::path::Path;
use std::error::Error;

use devise::ext::SpanDiagnosticExt;
use devise::syn::{self, Ident, LitStr};
use devise::proc_macro2::TokenStream;

pub fn _macro(input: proc_macro::TokenStream) -> devise::Result<TokenStream> {
    let root_glob = syn::parse::<LitStr>(input.into())?;
    let tests = entry_to_tests(&root_glob)
        .map_err(|e| root_glob.span().error(format!("failed to read: {}", e)))?;

    Ok(quote!(#(#tests)*))
}

fn entry_to_tests(root_glob: &LitStr) -> Result<Vec<TokenStream>, Box<dyn Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("MANIFEST_DIR");
    let full_glob = Path::new(&manifest_dir).join(&root_glob.value()).display().to_string();

    let mut tests = vec![];
    for path in glob::glob(&full_glob).map_err(Box::new)? {
        let path = path.map_err(Box::new)?;
        let name = path.file_name()
            .and_then(|f| f.to_str())
            .map(|name| name.trim_matches(|c| char::is_numeric(c) || c == '-')
                .replace(|c| c == '-' || c == '.', "_"))
            .ok_or_else(|| "invalid file name")?;

        let ident = Ident::new(&name, root_glob.span());
        let full_path = Path::new(&manifest_dir).join(&path).display().to_string();
        tests.push(quote_spanned!(root_glob.span() => doc_comment::doctest!(#full_path, #ident);))
    }

    Ok(tests)
}
