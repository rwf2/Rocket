use proc_macro::TokenStream;
use devise::{Spanned, Result, ext::SpanDiagnosticExt};

use crate::syn;

#[derive(Debug)]
struct DatabaseInvocation {
    /// The attributes on the attributed structure.
    attrs: Vec<syn::Attribute>,
    /// The name of the structure on which `#[database(..)] struct This(..)` was invoked.
    type_name: syn::Ident,
    /// The visibility of the structure on which `#[database(..)] struct This(..)` was invoked.
    visibility: syn::Visibility,
    /// The database name as passed in via #[database('database name')].
    db_name: String,
    /// The type inside the structure: struct MyDb(ThisType).
    connection_type: syn::Type,
}

const EXAMPLE: &str = "example: `struct MyDatabase(diesel::SqliteConnection);`";
const ONLY_ON_STRUCTS_MSG: &str = "`database` attribute can only be used on structs";
const ONLY_UNNAMED_FIELDS: &str = "`database` attribute can only be applied to \
    structs with exactly one unnamed field";
const NO_GENERIC_STRUCTS: &str = "`database` attribute cannot be applied to structs \
    with generics";

fn parse_invocation(attr: TokenStream, input: TokenStream) -> Result<DatabaseInvocation> {
    let attr_stream2 = crate::proc_macro2::TokenStream::from(attr);
    let string_lit = crate::syn::parse2::<syn::LitStr>(attr_stream2)?;

    let input = crate::syn::parse::<syn::DeriveInput>(input).unwrap();
    if !input.generics.params.is_empty() {
        return Err(input.generics.span().error(NO_GENERIC_STRUCTS));
    }

    let structure = match input.data {
        syn::Data::Struct(s) => s,
        _ => return Err(input.span().error(ONLY_ON_STRUCTS_MSG))
    };

    let inner_type = match structure.fields {
        syn::Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            let first = fields.unnamed.first().expect("checked length");
            first.ty.clone()
        }
        _ => return Err(structure.fields.span().error(ONLY_UNNAMED_FIELDS).help(EXAMPLE))
    };

    Ok(DatabaseInvocation {
        attrs: input.attrs,
        type_name: input.ident,
        visibility: input.vis,
        db_name: string_lit.value(),
        connection_type: inner_type,
    })
}

#[allow(non_snake_case)]
pub fn database_attr(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let invocation = parse_invocation(attr, input)?;

    // Store everything we're going to need to generate code.
    let conn_type = &invocation.connection_type;
    let name = &invocation.db_name;
    let attrs = &invocation.attrs;
    let guard_type = &invocation.type_name;
    let vis = &invocation.visibility;
    let fairing_name = format!("'{}' Database Pool", name);
    let span = conn_type.span();

    // A few useful paths.
    let root = quote_spanned!(span => ::rocket_sync_db_pools);
    let rocket = quote!(#root::rocket);

    let request_guard_type = quote_spanned! { span =>
        #(#attrs)* #vis struct #guard_type(#root::Connection<Self, #conn_type>);
    };

    let pool = quote_spanned!(span => #root::ConnectionPool<Self, #conn_type>);
    let conn = quote_spanned!(span => #root::Connection<Self, #conn_type>);

    Ok(quote! {
        #request_guard_type

        impl #guard_type {
            /// Returns a fairing that initializes the database connection pool.
            pub fn fairing() -> impl #rocket::fairing::Fairing {
                <#pool>::fairing(#fairing_name, #name)
            }

            /// Returns an opaque type that represents the connection pool
            /// backing connections of type `Self`.
            pub fn pool<P: #rocket::Phase>(__rocket: &#rocket::Rocket<P>) -> Option<&#pool> {
                <#pool>::pool(&__rocket)
            }

            /// Runs the provided function `__f` in an async-safe blocking
            /// thread.
            pub async fn run<F, R>(&self, __f: F) -> R
                where F: FnOnce(&mut #conn_type) -> R + Send + 'static,
                      R: Send + 'static,
            {
                self.0.run(__f).await
            }

            /// Retrieves a connection of type `Self` from the `rocket` instance.
            pub async fn get_one<P: #rocket::Phase>(__rocket: &#rocket::Rocket<P>) -> Option<Self> {
                <#pool>::get_one(&__rocket).await.map(Self)
            }
        }

        #[#rocket::async_trait]
        impl<'r> #rocket::request::FromRequest<'r> for #guard_type {
            type Error = ();

            async fn from_request(
                __r: &'r #rocket::request::Request<'_>
            ) -> #rocket::request::Outcome<Self, ()> {
                <#conn>::from_request(__r).await.map(Self)
            }
        }

        impl #rocket::Sentinel for #guard_type {
            fn abort(__r: &#rocket::Rocket<#rocket::Ignite>) -> bool {
                <#conn>::abort(__r)
            }
        }
    }.into())
}
