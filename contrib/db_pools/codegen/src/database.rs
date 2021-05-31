use proc_macro::TokenStream;

use syn::{Fields, Data, Type, DeriveInput, Ident, Meta, NestedMeta, Lit};
use syn::spanned::Spanned;

use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};

#[derive(Debug)]
struct DatabaseInvocation {
    /// The name of the structure on which `#[derive(Database)] struct This(..)` was invoked.
    struct_name: Ident,
    /// The database name as passed in via `#[database("database name")]`.
    db_name: String,
    /// The type inside the structure: `struct MyDb(ThisType)`.
    pool_type: Type,
}

const EXAMPLE: &str = "example: `struct MyDatabase(sqlx::SqlitePool);`";
const ONE_DATABASE_ATTR: &str = "`Database` derive requires exactly one \
    `#[database(name=\"\")] attribute`";
const ONLY_ON_STRUCTS_MSG: &str = "`Database` derive can only be used on structs";
const ONE_UNNAMED_FIELD: &str = "`Database` derive can only be applied to \
    structs with exactly one unnamed field";
const NO_GENERIC_STRUCTS: &str = "`Database` derive cannot be applied to structs \
    with generics";

fn parse_invocation(input: TokenStream) -> Result<DatabaseInvocation, Diagnostic> {
    let input = syn::parse::<DeriveInput>(input).unwrap();

    let db_attrs = input.attrs
        .iter()
        .filter(|a| a.path.get_ident().map_or(false, |i| i == "database"))
        .collect::<Vec<_>>();

    let db_attr = if db_attrs.len() == 1 {
        db_attrs.first().expect("checked length")
    } else {
        return Err(input.span().error(ONE_DATABASE_ATTR));
    };

    let maybe_db_name = match db_attr.parse_meta()? {
        Meta::List(list) if list.nested.len() == 1 => {
            match &list.nested[0] {
                NestedMeta::Meta(Meta::NameValue(nameval))
                    if nameval.path.get_ident().map_or(false, |i| i == "name") =>
                {
                    match &nameval.lit {
                        Lit::Str(litstr) => Some(litstr.value()),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    };

    let db_name = match maybe_db_name {
        Some(n) => n,
        None => return Err(input.span().error(ONE_DATABASE_ATTR)),
    };

    let structure = match input.data {
        Data::Struct(s) => s,
        _ => return Err(input.span().error(ONLY_ON_STRUCTS_MSG)),
    };

    if !input.generics.params.is_empty() {
        return Err(input.generics.span().error(NO_GENERIC_STRUCTS));
    }

    let pool_type = match structure.fields {
        Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            let first = fields.unnamed.first().expect("checked length");
            first.ty.clone()
        }
        _ => return Err(structure.fields.span().error(ONE_UNNAMED_FIELD).help(EXAMPLE))
    };

    Ok(DatabaseInvocation {
        struct_name: input.ident,
        db_name,
        pool_type,
    })
}

#[allow(non_snake_case)]
pub fn derive_database(input: TokenStream) -> Result<TokenStream, Diagnostic> {
    // Store everything we're going to need to generate code.
    let DatabaseInvocation {
        ref struct_name,
        ref db_name,
        ref pool_type,
    } = parse_invocation(input)?;

    let span = pool_type.span().into();

    // A few useful paths.
    let krate = quote_spanned!(span => ::rocket_db_pools);

    Ok(quote! {
        impl #krate::Database for #struct_name {
            const NAME: &'static str = #db_name;
            type Pool = #pool_type;
            fn fairing() -> #krate::Fairing<Self> { #krate::Fairing::new(Self) }
            fn pool(&self) -> &Self::Pool { &self.0 }
        }
    }.into())
}
