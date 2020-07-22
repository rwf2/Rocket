use proc_macro::TokenStream;
use devise::{Spanned, Result, ext::SpanDiagnosticExt};

use crate::syn::{DataStruct, Fields, Data, Type, LitStr, DeriveInput, Ident, Visibility};

#[derive(Debug)]
struct DatabaseInvocation {
    /// The name of the structure on which `#[database(..)] struct This(..)` was invoked.
    type_name: Ident,
    /// The visibility of the structure on which `#[database(..)] struct This(..)` was invoked.
    visibility: Visibility,
    /// The database name as passed in via #[database('database name')].
    db_name: String,
    /// The entire structure that the `database` attribute was called on.
    structure: DataStruct,
    /// The type inside the structure: struct MyDb(ThisType).
    connection_type: Type,
}

const EXAMPLE: &str = "example: `struct MyDatabase(diesel::SqliteConnection);`";
const ONLY_ON_STRUCTS_MSG: &str = "`database` attribute can only be used on structs";
const ONLY_UNNAMED_FIELDS: &str = "`database` attribute can only be applied to \
    structs with exactly one unnamed field";
const NO_GENERIC_STRUCTS: &str = "`database` attribute cannot be applied to structs \
    with generics";

fn parse_invocation(attr: TokenStream, input: TokenStream) -> Result<DatabaseInvocation> {
    let attr_stream2 = crate::proc_macro2::TokenStream::from(attr);
    let string_lit = crate::syn::parse2::<LitStr>(attr_stream2)?;

    let input = crate::syn::parse::<DeriveInput>(input).unwrap();
    if !input.generics.params.is_empty() {
        return Err(input.generics.span().error(NO_GENERIC_STRUCTS));
    }

    let structure = match input.data {
        Data::Struct(s) => s,
        _ => return Err(input.span().error(ONLY_ON_STRUCTS_MSG))
    };

    let inner_type = match structure.fields {
        Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            let first = fields.unnamed.first().expect("checked length");
            first.ty.clone()
        }
        _ => return Err(structure.fields.span().error(ONLY_UNNAMED_FIELDS).help(EXAMPLE))
    };

    Ok(DatabaseInvocation {
        type_name: input.ident,
        visibility: input.vis,
        db_name: string_lit.value(),
        structure: structure,
        connection_type: inner_type,
    })
}

#[allow(non_snake_case)]
pub fn database_attr(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let invocation = parse_invocation(attr, input)?;

    // Store everything we're going to need to generate code.
    let conn_type = &invocation.connection_type;
    let name = &invocation.db_name;
    let guard_type = &invocation.type_name;
    let vis = &invocation.visibility;
    let fairing_name = format!("'{}' Database Pool", name);
    let span = conn_type.span().into();

    // A few useful paths.
    let databases = quote_spanned!(span => ::rocket_contrib::databases);
    let request = quote!(::rocket::request);

    let generated_types = quote_spanned! { span =>
        /// The request guard type.
        #vis struct #guard_type(#databases::Connection<Self, #conn_type>);
    };

    Ok(quote! {
        #generated_types

        impl #guard_type {
            /// Returns a fairing that initializes the associated database
            /// connection pool.
            pub fn fairing() -> impl ::rocket::fairing::Fairing {
                <#databases::ConnectionPool<Self, #conn_type>>::fairing(#fairing_name, #name)
            }

            /// Retrieves a connection of type `Self` from the `rocket`
            /// instance. Returns `Some` as long as `Self::fairing()` has been
            /// attached.
            pub async fn get_one(cargo: &::rocket::Cargo) -> Option<Self> {
                <#databases::ConnectionPool<Self, #conn_type>>::get_one(cargo).await.map(Self)
            }

            /// Runs the provided closure on a thread from a threadpool. The
            /// closure will be passed an `&mut r2d2::PooledConnection`.
            /// `.await`ing the return value of this function yields the value
            /// returned by the closure.
            pub async fn run<F, R>(self, f: F) -> R
            where
                F: FnOnce(&mut #conn_type) -> R + Send + 'static,
                R: Send + 'static,
            {
                self.0.run(f).await
            }

            /// Asynchronously acquires another connection from the connection pool.
            pub async fn clone(&mut self) -> ::std::result::Result<Self, ()> {
                self.0.clone().await.map(Self)
            }
        }

        #[::rocket::async_trait]
        impl<'a, 'r> #request::FromRequest<'a, 'r> for #guard_type {
            type Error = ();

            async fn from_request(request: &'a #request::Request<'r>) -> #request::Outcome<Self, ()> {
                <#databases::Connection<Self, #conn_type>>::from_request(request).await.map(Self)
            }
        }
    }.into())
}
