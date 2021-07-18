use proc_macro::TokenStream;

use devise::{DeriveGenerator, FromMeta, MapperBuild, Support, ValidatorBuild};
use devise::proc_macro2_diagnostics::SpanDiagnosticExt;
use devise::syn::{self, spanned::Spanned};

const ONE_DATABASE_ATTR: &str = "missing `#[database(\"name\")]` attribute";
const ONE_UNNAMED_FIELD: &str = "struct must have exactly one unnamed field";

#[derive(Debug, FromMeta)]
struct DatabaseAttribute {
    #[meta(naked)]
    name: String,
}

pub fn derive_database(input: TokenStream) -> TokenStream {
    DeriveGenerator::build_for(input, quote!(impl rocket_db_pools::Database))
        .support(Support::TupleStruct)
        .validator(ValidatorBuild::new()
            .struct_validate(|_, s| {
                if s.fields.len() == 1 {
                    Ok(())
                } else {
                    Err(s.span().error(ONE_UNNAMED_FIELD))
                }
            })
        )
        .outer_mapper(MapperBuild::new()
            .struct_map(|_, s| {
                let pool_type = match &s.fields {
                    syn::Fields::Unnamed(f) => &f.unnamed[0].ty,
                    _ => unreachable!("Support::TupleStruct"),
                };

                let decorated_type = &s.ident;
                let db_ty = quote_spanned!(decorated_type.span() =>
                    <#decorated_type as rocket_db_pools::Database>
                );

                quote_spanned! { decorated_type.span() =>
                    impl From<#pool_type> for #decorated_type {
                        fn from(pool: #pool_type) -> Self {
                            Self(pool)
                        }
                    }

                    impl std::ops::Deref for #decorated_type {
                        type Target = #pool_type;

                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    impl std::ops::DerefMut for #decorated_type {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.0
                        }
                    }

                    #[rocket::async_trait]
                    impl<'r> rocket::request::FromRequest<'r> for &'r #decorated_type {
                        type Error = ();

                        async fn from_request(
                            req: &'r rocket::request::Request<'_>
                        ) -> rocket::request::Outcome<Self, Self::Error> {
                            match #db_ty::fetch(req.rocket()) {
                                Some(db) => rocket::outcome::Outcome::Success(db),
                                None => rocket::outcome::Outcome::Failure((
                                    rocket::http::Status::InternalServerError, ()))
                            }
                        }
                    }

                    impl rocket::Sentinel for &#decorated_type {
                        fn abort(rocket: &rocket::Rocket<rocket::Ignite>) -> bool {
                            #db_ty::fetch(rocket).is_none()
                        }
                    }
                }
            })
        )
        .outer_mapper(quote!(#[rocket::async_trait]))
        .inner_mapper(MapperBuild::new()
            .try_struct_map(|_, s| {
                let db_name = DatabaseAttribute::one_from_attrs("database", &s.attrs)?
                    .map(|attr| attr.name)
                    .ok_or_else(|| s.span().error(ONE_DATABASE_ATTR))?;

                let fairing_name = format!("'{}' Database Pool", db_name);

                let pool_type = match &s.fields {
                    syn::Fields::Unnamed(f) => &f.unnamed[0].ty,
                    _ => unreachable!("Support::TupleStruct"),
                };

                Ok(quote_spanned! { pool_type.span() =>
                    type Pool = #pool_type;

                    const NAME: &'static str = #db_name;

                    fn init() -> rocket_db_pools::Initializer<Self> {
                        rocket_db_pools::Initializer::with_name(#fairing_name)
                    }
                })
            })
        )
        .to_tokens()
}
