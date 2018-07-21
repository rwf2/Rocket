use proc_macro::{TokenStream, Diagnostic};
use syn::{DataStruct, Fields, Data, Type, LitStr, DeriveInput, Ident, Visibility};
use spanned::Spanned;

type Result<T> = ::std::result::Result<T, Diagnostic>;

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
const ONLY_UNNAMED_FIELDS: &str = "`database` attribute can only be applied to structs with \
    exactly one unnamed field";
const NO_GENERIC_STRUCTS: &str = "`database` attribute cannot be applied to a struct with a \
    generic type";

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

    let connection_type = &invocation.connection_type;
    let database_name = &invocation.db_name;
    let request_guard_type = &invocation.type_name;
    let request_guard_vis = &invocation.visibility;
    let pool_type = Ident::new(&format!("{}Pool", request_guard_type), request_guard_type.span());

    let tokens = quote! {
        #request_guard_vis struct #request_guard_type(
            pub ::rocket_contrib::databases::r2d2::PooledConnection<<#connection_type as ::rocket_contrib::databases::Poolable>::Manager>
        );
        #request_guard_vis struct #pool_type(
            ::rocket_contrib::databases::r2d2::Pool<<#connection_type as ::rocket_contrib::databases::Poolable>::Manager>
        );

        impl #request_guard_type {
            pub fn fairing() -> impl ::rocket::fairing::Fairing {
                use ::rocket_contrib::databases::Poolable;

                ::rocket::fairing::AdHoc::on_attach(|rocket| {
                    let pool = ::rocket_contrib::databases::database_config(#database_name, rocket.config())
                        .map(#connection_type::pool);

                    match pool {
                        Ok(Ok(p)) => Ok(rocket.manage(#pool_type(p))),
                        Err(config_error) => {
                            ::rocket::logger::log_err(&format!("Error while instantiating database: '{}': {}", #database_name, config_error));
                            Err(rocket)
                        },
                        Ok(Err(pool_error)) => {
                            ::rocket::logger::log_err(&format!("Error initializing pool for '{}': {:?}", #database_name, pool_error));
                            Err(rocket)
                        },
                    }
                })
            }

            /// Retrieves a connection of type `Self` from the `rocket` instance. Returns `Some` as long as
            /// `Self::fairing()` has been attached and there is at least one connection in the pool.
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

        impl<'a, 'r> ::rocket::request::FromRequest<'a, 'r> for #request_guard_type {
            type Error = ();

            fn from_request(request: &'a ::rocket::request::Request<'r>) -> ::rocket::request::Outcome<Self, Self::Error> {
                let pool = request.guard::<::rocket::State<#pool_type>>()?;

                match pool.0.get() {
                    Ok(conn) => ::rocket::Outcome::Success(#request_guard_type(conn)),
                    Err(_) => ::rocket::Outcome::Failure((::rocket::http::Status::ServiceUnavailable, ())),
                }
            }
        }
    };

    Ok(tokens.into())
}
