#![feature(proc_macro_diagnostic)]
#![feature(crate_visibility_modifier)]
#![recursion_limit="128"]

#[macro_use] extern crate quote;
#[macro_use] extern crate derive_utils;
extern crate proc_macro;
extern crate rocket_http;

mod derive;
mod http_codegen;
mod prefixing_vec;

crate use derive_utils::{syn, proc_macro2};

use proc_macro::TokenStream;

#[proc_macro_derive(FromFormValue, attributes(form))]
pub fn derive_from_form_value(input: TokenStream) -> TokenStream {
    derive::from_form_value::derive_from_form_value(input)
}

#[proc_macro_derive(FromForm, attributes(form))]
pub fn derive_from_form(input: TokenStream) -> TokenStream {
    derive::from_form::derive_from_form(input)
}

#[proc_macro_derive(Responder, attributes(response))]
pub fn derive_responder(input: TokenStream) -> TokenStream {
    derive::responder::derive_responder(input)
}

const ROUTE_STRUCT_PREFIX: &'static str = "static_rocket_route_info_for_";
#[proc_macro]
pub fn rocket_routes_internal(input: TokenStream) -> TokenStream {
    prefixing_vec::prefixing_vec_macro(ROUTE_STRUCT_PREFIX, |path| {
        quote!(::rocket::Route::from(&#path))
    }, input)
}


const CATCH_STRUCT_PREFIX: &'static str = "static_rocket_catch_info_for_";
#[proc_macro]
pub fn rocket_catchers_internal(input: TokenStream) -> TokenStream {
    prefixing_vec::prefixing_vec_macro(CATCH_STRUCT_PREFIX, |path| {
        quote!(::rocket::Catcher::from(&#path))
    }, input)
}
