use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream, discouraged::Speculative};

#[derive(Debug)]
pub enum Input {
    Type(syn::Type, Option<(syn::Token![+], syn::Lifetime)>),
    Tokens(TokenStream)
}

#[derive(Debug)]
struct Invocation {
    ty_stream_ty: syn::Path,
    stream_mac: syn::Path,
    stream_trait: syn::Path,
    input: Input,
}

/// Reinterpret a `T + '_` (without the `dyn`) for `impl Stream<T> + '_`.
fn trait_obj_recast(ty: &syn::Type) -> Option<(syn::Type, syn::Token![+], syn::Lifetime)> {
    let bounds = match ty {
        syn::Type::TraitObject(t) if t.dyn_token.is_none() => &t.bounds,
        _ => return None
    };

    let mut bounds = bounds.pairs();
    let (first, second) = (bounds.next()?, bounds.next()?);
    let plus = **first.punct().expect("have two so have punct");

    let first = first.value();
    let real_ty = syn::parse2(quote!(#first)).ok()?;
    let lifetime = match second.value() {
        syn::TypeParamBound::Lifetime(lt) => lt.clone(),
        _ => return None,
    };

    Some((real_ty, plus, lifetime))
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let fork = input.fork();
        if let Ok(mut ty) = fork.parse() {
            // If there's an extra + '_, use it in the reinterpretation.
            let mut bound = match fork.parse() {
                Ok(plus) => Some((plus, fork.parse()?)),
                _ => None,
            };

            // We might miss `A + '_`. Check if we did.
            if let Some((real_ty, plus, lt)) = trait_obj_recast(&ty) {
                ty = real_ty;
                bound = Some((plus, lt));
            }

            // If we have nothing else to parse, this must have been a type.
            if fork.is_empty() {
                input.advance_to(&fork);
                Ok(Input::Type(ty, bound))
            } else {
                Ok(Input::Tokens(input.parse()?))
            }
        } else {
            Ok(Input::Tokens(input.parse()?))
        }
    }
}

impl Parse for Invocation {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Invocation {
            ty_stream_ty: (input.parse()?, input.parse::<syn::Token![,]>()?).0,
            stream_mac: (input.parse()?, input.parse::<syn::Token![,]>()?).0,
            stream_trait: (input.parse()?, input.parse::<syn::Token![,]>()?).0,
            input: input.parse()?,
        })
    }
}

/// This macro exists because we want to disambiguate between input of a type
/// and input of an expression that looks like a type. `macro_rules` matches
/// eagerly on a single token, so something like `foo!(for x in 0..10 {})` will
/// match a `($ty)` branch as will anything that starts with a path.
pub fn _macro(input: proc_macro::TokenStream) -> devise::Result<TokenStream> {
    let tokens = proc_macro2::TokenStream::from(input);
    let i: Invocation = syn::parse2(tokens)?;
    let (s_ty, mac, s_trait) = (i.ty_stream_ty, i.stream_mac, i.stream_trait);
    let tokens = match i.input {
        Input::Tokens(tt) => quote!(#s_ty::from(#mac!(#tt))),
        Input::Type(ty, Some((p, l))) => quote!(#s_ty<impl #s_trait<Item = #ty> #p #l>),
        Input::Type(ty, None) => quote!(#s_ty<impl #s_trait<Item = #ty>>),
    };

    Ok(tokens)
}
