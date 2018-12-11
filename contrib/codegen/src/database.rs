use proc_macro::TokenStream;
use devise::{Spanned, Result};
use syn::{DataStruct, Fields, Data, Type, LitStr, DeriveInput, Ident, Visibility};

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
const NO_GENERIC_STRUCTS: &str = "`database` attribute cannot be applied to a struct \
    with a generic type";

fn parse_invocation(attr: TokenStream, input: TokenStream) -> Result<DatabaseInvocation> {
    let attr_stream2 = ::proc_macro2::TokenStream::from(attr);
    let attr_span = attr_stream2.span();
    let string_lit = ::syn::parse2::<LitStr>(attr_stream2)
        .map_err(|_| attr_span.error("expected string literal"))?;

    let input = ::syn::parse::<DeriveInput>(input).unwrap();
    if !input.generics.params.is_empty() {
        return Err(input.span().error(NO_GENERIC_STRUCTS));
    }

    let structure = match input.data {
        Data::Struct(s) => s,
        _ => return Err(input.span().error(ONLY_ON_STRUCTS_MSG))
    };

    let inner_type = match structure.fields {
        Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            let first = fields.unnamed.first().expect("checked length");
            first.value().ty.clone()
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

pub fn database_attr(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let invocation = parse_invocation(attr, input)?;

    // Store everything we're going to need to generate code.
    let connection_type = &invocation.connection_type;
    let name = &invocation.db_name;
    let request_guard_type = &invocation.type_name;
    let vis = &invocation.visibility;
    let pool_type = Ident::new(&format!("{}Pool", request_guard_type), request_guard_type.span());
    let fairing_name = format!("'{}' Database Pool", name);

    // A few useful paths.
    let databases = quote!(::rocket_contrib::databases);
    let r2d2 = quote!(#databases::r2d2);
    let request = quote!(::rocket::request);

    Ok(quote! {
        /// The request guard type.
        #vis struct #request_guard_type(
            pub #r2d2::PooledConnection<<#connection_type as #databases::Poolable>::Manager>
        );

        /// The pool type.
        #vis struct #pool_type(
            #r2d2::Pool<<#connection_type as #databases::Poolable>::Manager>
        );

        impl #request_guard_type {
            /// Returns a fairing that initializes the associated database
            /// connection pool.
            pub fn fairing() -> impl ::rocket::fairing::Fairing {
                use #databases::Poolable;

                ::rocket::fairing::AdHoc::on_attach(#fairing_name, |rocket| {
                    let pool = #databases::database_config(#name, rocket.config())
                        .map(<#connection_type>::pool);

                    match pool {
                        Ok(Ok(p)) => Ok(rocket.manage(#pool_type(p))),
                        Err(config_error) => {
                            ::rocket::logger::error(
                                &format!("Database configuration failure: '{}'", #name));
                            ::rocket::logger::error_(&format!("{}", config_error));
                            Err(rocket)
                        },
                        Ok(Err(pool_error)) => {
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
            /// attached and there is at least one connection in the pool.
            pub fn get_one(rocket: &::rocket::Rocket) -> Option<Self> {
                rocket.state::<#pool_type>()
                    .and_then(|pool| pool.0.get().ok())
                    .map(#request_guard_type)
            }
        }

        impl ::std::ops::Deref for #request_guard_type {
            type Target = #connection_type;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::ops::DerefMut for #request_guard_type {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<'a, 'r> #request::FromRequest<'a, 'r> for #request_guard_type {
            type Error = ();

            fn from_request(request: &'a #request::Request<'r>) -> #request::Outcome<Self, ()> {
                use ::rocket::{Outcome, http::Status};
                let pool = request.guard::<::rocket::State<#pool_type>>()?;

                match pool.0.get() {
                    Ok(conn) => Outcome::Success(#request_guard_type(conn)),
                    Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
                }
            }
        }
    }.into())
}
