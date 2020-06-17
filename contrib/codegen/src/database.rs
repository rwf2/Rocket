use proc_macro::TokenStream;
use devise::{Spanned, Result};
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
    let attr_span = attr_stream2.span();
    let string_lit = crate::syn::parse2::<LitStr>(attr_stream2)
        .map_err(|_| attr_span.error("expected string literal"))?;

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
    let pool_type = Ident::new(&format!("{}Pool", guard_type), guard_type.span());
    let fairing_name = format!("'{}' Database Pool", name);
    let span = conn_type.span().into();

    // A few useful paths.
    let databases = quote_spanned!(span => ::rocket_contrib::databases);
    let Poolable = quote_spanned!(span => #databases::Poolable);
    let r2d2 = quote_spanned!(span => #databases::r2d2);
    let spawn_blocking = quote_spanned!(span => #databases::spawn_blocking);
    let request = quote!(::rocket::request);
    let Arc = quote!(::std::sync::Arc);
    let Semaphore = quote!(::rocket::tokio::sync::Semaphore);

    let generated_types = quote_spanned! { span =>
        /// The request guard type.
        // TODO: Nest this and pool_type in another module to prevent direct
        // access to fields by user code.
        #vis struct #guard_type(#r2d2::Pool<<#conn_type as #Poolable>::Manager>, #Arc<#Semaphore>);

        /// The pool type.
        #vis struct #pool_type(#r2d2::Pool<<#conn_type as #Poolable>::Manager>, #Arc<#Semaphore>);
    };

    Ok(quote! {
        #generated_types

        impl #guard_type {
            /// Returns a fairing that initializes the associated database
            /// connection pool.
            pub fn fairing() -> impl ::rocket::fairing::Fairing {
                use #databases::Poolable;

                ::rocket::fairing::AdHoc::on_attach(#fairing_name, |mut rocket| async {
                    let config = #databases::database_config(#name, rocket.config().await);
                    let pool = config.map(|c| (c.pool_size, <#conn_type>::pool(c)));

                    match pool {
                        Ok((size, Ok(p))) => Ok(rocket.manage(#pool_type(p, #Arc::new(#Semaphore::new(size as usize))))),
                        Err(config_error) => {
                            ::rocket::logger::error(
                                &format!("Database configuration failure: '{}'", #name));
                            ::rocket::logger::error_(&format!("{}", config_error));
                            Err(rocket)
                        },
                        Ok((_, Err(pool_error))) => {
                            ::rocket::logger::error(
                                &format!("Failed to initialize pool for '{}'", #name));
                            ::rocket::logger::error_(&format!("{:?}", pool_error));
                            Err(rocket)
                        },
                    }
                })
            }

            /// Retrieves a connection of type `Self` from the `rocket`
            /// instance. Returns `Some` as long as `Self::fairing()` has been
            /// attached.
            pub fn get_one(cargo: &::rocket::Cargo) -> Option<Self> {
                cargo.state::<#pool_type>()
                    .map(|c| #guard_type(c.0.clone(), c.1.clone()))
            }

            /// Runs the provided closure on a blocking threadpool. The closure
            /// will be passed an `&mut r2d2::PooledConnection`, and `.await`ing
            /// this function will provide whatever value is returned from the
            /// closure.
            pub async fn run<F, R>(&self, f: F) -> R
            where
                F: FnOnce(&mut #r2d2::PooledConnection<<#conn_type as #Poolable>::Manager>) -> R + Send + 'static,
                R: Send + 'static,
            {
                let _permit = self.1.acquire().await;
                let pool = self.0.clone();
                let ret = #spawn_blocking(move || {
                    let mut conn = pool.get().expect("TODO");
                    let ret = f(&mut conn);
                    ret
                }).await.expect("failed to spawn a blocking task to use a pooled connection");
                ret
            }
        }

        #[::rocket::async_trait]
        impl<'a, 'r> #request::FromRequest<'a, 'r> for #guard_type {
            type Error = ();

            async fn from_request(request: &'a #request::Request<'r>) -> #request::Outcome<Self, ()> {
                use ::rocket::{Outcome, http::Status};
                let inner = ::rocket::try_outcome!(request.guard::<::rocket::State<'_, #pool_type>>().await);
                Outcome::Success(#guard_type(inner.0.clone(), inner.1.clone()))
            }
        }
    }.into())
}
