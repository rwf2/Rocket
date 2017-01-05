use std::collections::HashSet;
use std::fmt::Display;

use ::{ROUTE_STRUCT_PREFIX, ROUTE_FN_PREFIX, PARAM_PREFIX};
use utils::{emit_item, span, sep_by_tok, option_as_expr, strip_ty_lifetimes};
use utils::{SpanExt, IdentExt, ArgExt};
use parser::{Param, RouteParams};

use syntax::codemap::{Span, Spanned};
use syntax::tokenstream::TokenTree;
use syntax::ast::{Arg, Ident, Stmt, Expr, MetaItem, Path};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::parse::token;
use syntax::ptr::P;

use rocket::http::{Method, ContentType};

fn method_to_path(ecx: &ExtCtxt, method: Method) -> Path {
    quote_enum!(ecx, method => ::rocket::http::Method {
        Options, Get, Post, Put, Delete, Head, Trace, Connect, Patch;
    })
}

fn content_type_to_expr(ecx: &ExtCtxt, ct: Option<ContentType>) -> Option<P<Expr>> {
    ct.map(|ct| {
        let (top, sub) = (ct.ttype.as_str(), ct.subtype.as_str());
        quote_expr!(ecx, ::rocket::http::ContentType {
            ttype: ::rocket::http::ascii::UncasedAscii {
                string: ::std::borrow::Cow::Borrowed($top)
            },
            subtype: ::rocket::http::ascii::UncasedAscii {
                string: ::std::borrow::Cow::Borrowed($sub)
            },
            params: None
        })
    })
}

trait RouteGenerateExt {
    fn gen_form(&self, &ExtCtxt, Option<&Spanned<Ident>>, P<Expr>) -> Option<Stmt>;
    fn missing_declared_err<T: Display>(&self, ecx: &ExtCtxt, arg: &Spanned<T>);

    fn generate_data_statement(&self, ecx: &ExtCtxt) -> Option<Stmt>;
    fn generate_query_statement(&self, ecx: &ExtCtxt) -> Option<Stmt>;
    fn generate_param_statements(&self, ecx: &ExtCtxt) -> Vec<Stmt>;
    fn generate_fn_arguments(&self, ecx: &ExtCtxt) -> Vec<TokenTree>;
    fn explode(&self, ecx: &ExtCtxt) -> (&String, Path, P<Expr>, P<Expr>);
}

impl RouteGenerateExt for RouteParams {
    fn missing_declared_err<T: Display>(&self, ecx: &ExtCtxt, arg: &Spanned<T>) {
        let fn_span = self.annotated_fn.span();
        let msg = format!("'{}' is declared as an argument...", arg.node);
        ecx.span_err(arg.span, &msg);
        ecx.span_err(fn_span, "...but isn't in the function signature.");
    }

    fn gen_form(&self, ecx: &ExtCtxt, param: Option<&Spanned<Ident>>,
                form_string: P<Expr>) -> Option<Stmt> {
        let arg = param.and_then(|p| self.annotated_fn.find_input(&p.node.name));
        if param.is_none() {
            return None;
        } else if arg.is_none() {
            self.missing_declared_err(ecx, &param.unwrap());
            return None;
        }

        let arg = arg.unwrap();
        let name = arg.ident().expect("form param identifier").prepend(PARAM_PREFIX);
        let ty = strip_ty_lifetimes(arg.ty.clone());
        Some(quote_stmt!(ecx,
            let $name: $ty =
                match ::rocket::request::FromForm::from_form_string($form_string) {
                    Ok(v) => v,
                    Err(_) => return ::rocket::Outcome::Forward(_data)
                };
        ).expect("form statement"))
    }

    fn generate_data_statement(&self, ecx: &ExtCtxt) -> Option<Stmt> {
        let param = self.data_param.as_ref().map(|p| &p.value);
        let arg = param.and_then(|p| self.annotated_fn.find_input(&p.node.name));
        if param.is_none() {
            return None;
        } else if arg.is_none() {
            self.missing_declared_err(ecx, &param.unwrap());
            return None;
        }

        let arg = arg.unwrap();
        let name = arg.ident().expect("form param identifier").prepend(PARAM_PREFIX);
        let ty = strip_ty_lifetimes(arg.ty.clone());
        Some(quote_stmt!(ecx,
            let $name: $ty =
                match ::rocket::data::FromData::from_data(&_req, _data) {
                    ::rocket::Outcome::Success(d) => d,
                    ::rocket::Outcome::Forward(d) =>
                        return ::rocket::Outcome::Forward(d),
                    ::rocket::Outcome::Failure((code, _)) => {
                        return ::rocket::Outcome::Failure(code);
                    }
                };
        ).expect("data statement"))
    }

    fn generate_query_statement(&self, ecx: &ExtCtxt) -> Option<Stmt> {
        let param = self.query_param.as_ref();
        let expr = quote_expr!(ecx,
           match _req.uri().query() {
               Some(query) => query,
               None => return ::rocket::Outcome::Forward(_data)
           }
        );

        self.gen_form(ecx, param, expr)
    }

    // TODO: Add some kind of logging facility in Rocket to get be able to log
    // an error/debug message if parsing a parameter fails.
    fn generate_param_statements(&self, ecx: &ExtCtxt) -> Vec<Stmt> {
        let mut fn_param_statements = vec![];

        // Generate a statement for every declared paramter in the path.
        let mut declared_set = HashSet::new();
        for (i, param) in self.path_params(ecx).enumerate() {
            declared_set.insert(param.ident().name.clone());
            let ty = match self.annotated_fn.find_input(&param.ident().name) {
                Some(arg) => strip_ty_lifetimes(arg.ty.clone()),
                None => {
                    self.missing_declared_err(ecx, param.inner());
                    continue;
                }
            };

            // Note: the `None` case shouldn't happen if a route is matched.
            let ident = param.ident().prepend(PARAM_PREFIX);
            let expr = match param {
                Param::Single(_) => quote_expr!(ecx, match _req.get_param_str($i) {
                    Some(s) => <$ty as ::rocket::request::FromParam>::from_param(s),
                    None => return ::rocket::Outcome::Forward(_data)
                }),
                Param::Many(_) => quote_expr!(ecx, match _req.get_raw_segments($i) {
                    Some(s) => <$ty as ::rocket::request::FromSegments>::from_segments(s),
                    None => return ::rocket::Outcome::Forward(_data)
                }),
            };

            let original_ident = param.ident();
            fn_param_statements.push(quote_stmt!(ecx,
                let $ident: $ty = match $expr {
                    Ok(v) => v,
                    Err(e) => {
                        println!("    => Failed to parse '{}': {:?}",
                                 stringify!($original_ident), e);
                        return ::rocket::Outcome::Forward(_data)
                    }
                };
            ).expect("declared param parsing statement"));
        }

        // A from_request parameter is one that isn't declared, data, or query.
        let from_request = |a: &&Arg| {
            if let Some(name) = a.name() {
                !declared_set.contains(name)
                    && self.data_param.as_ref().map_or(true, |p| {
                        !a.named(&p.value().name)
                    }) && self.query_param.as_ref().map_or(true, |p| {
                        !a.named(&p.node.name)
                    })
            } else {
                ecx.span_err(a.pat.span, "argument names must be identifiers");
                false
            }
        };

        // Generate the code for `from_request` parameters.
        let all = &self.annotated_fn.decl().inputs;
        for arg in all.iter().filter(from_request) {
            let ident = arg.ident().unwrap().prepend(PARAM_PREFIX);
            let ty = strip_ty_lifetimes(arg.ty.clone());
            fn_param_statements.push(quote_stmt!(ecx,
                let $ident: $ty = match
                        ::rocket::request::FromRequest::from_request(&_req) {
                    ::rocket::outcome::Outcome::Success(v) => v,
                    ::rocket::outcome::Outcome::Forward(_) =>
                        return ::rocket::Outcome::forward(_data),
                    ::rocket::outcome::Outcome::Failure((code, _)) => {
                        return ::rocket::Outcome::Failure(code)
                    },
                };
            ).expect("undeclared param parsing statement"));
        }

        fn_param_statements
    }

    fn generate_fn_arguments(&self, ecx: &ExtCtxt) -> Vec<TokenTree> {
        let args = self.annotated_fn.decl().inputs.iter()
            .filter_map(|a| a.ident())
            .map(|ident| ident.prepend(PARAM_PREFIX))
            .collect::<Vec<Ident>>();

        sep_by_tok(ecx, &args, token::Comma)
    }

    fn explode(&self, ecx: &ExtCtxt) -> (&String, Path, P<Expr>, P<Expr>) {
        let path = &self.path.node;
        let method = method_to_path(ecx, self.method.node);
        let format = self.format.as_ref().map(|kv| kv.value().clone());
        let content_type = option_as_expr(ecx, &content_type_to_expr(ecx, format));
        let rank = option_as_expr(ecx, &self.rank);

        (path, method, content_type, rank)
    }
}

// FIXME: Compilation fails when parameters have the same name as the function!
fn generic_route_decorator(known_method: Option<Spanned<Method>>,
                           ecx: &mut ExtCtxt,
                           sp: Span,
                           meta_item: &MetaItem,
                           annotated: &Annotatable,
                           push: &mut FnMut(Annotatable)) {
    // Parse the route and generate the code to create the form and param vars.
    let route = RouteParams::from(ecx, sp, known_method, meta_item, annotated);
    debug!("Route params: {:?}", route);

    let param_statements = route.generate_param_statements(ecx);
    let query_statement = route.generate_query_statement(ecx);
    let data_statement = route.generate_data_statement(ecx);
    let fn_arguments = route.generate_fn_arguments(ecx);

    // Generate and emit the wrapping function with the Rocket handler signature.
    let user_fn_name = route.annotated_fn.ident();
    let route_fn_name = user_fn_name.prepend(ROUTE_FN_PREFIX);
    emit_item(push, quote_item!(ecx,
         fn $route_fn_name<'_b>(_req: &'_b ::rocket::Request,  _data: ::rocket::Data)
                -> ::rocket::handler::Outcome<'_b> {
             $param_statements
             $query_statement
             $data_statement
             let responder = $user_fn_name($fn_arguments);
             ::rocket::handler::Outcome::of(responder)
         }
    ).unwrap());

    // Generate and emit the static route info that uses the just generated
    // function as its handler. A proper Rocket route will be created from this.
    let struct_name = user_fn_name.prepend(ROUTE_STRUCT_PREFIX);
    let (path, method, content_type, rank) = route.explode(ecx);
    emit_item(push, quote_item!(ecx,
        #[allow(non_upper_case_globals)]
        pub static $struct_name: ::rocket::StaticRouteInfo =
            ::rocket::StaticRouteInfo {
                method: $method,
                path: $path,
                handler: $route_fn_name,
                format: $content_type,
                rank: $rank,
            };
    ).unwrap());
}

pub fn route_decorator(ecx: &mut ExtCtxt,
                       sp: Span,
                       meta_item: &MetaItem,
                       annotated: &Annotatable,
                       push: &mut FnMut(Annotatable)) {
    generic_route_decorator(None, ecx, sp, meta_item, annotated, push);
}

macro_rules! method_decorator {
    ($name:ident, $method:ident) => (
        pub fn $name(ecx: &mut ExtCtxt, sp: Span, meta_item: &MetaItem,
                     annotated: &Annotatable, push: &mut FnMut(Annotatable)) {
            let i_sp = meta_item.span.shorten_to(stringify!($method).len());
            let method = Some(span(Method::$method, i_sp));
            generic_route_decorator(method, ecx, sp, meta_item, annotated, push);
        }
    )
}

method_decorator!(get_decorator, Get);
method_decorator!(put_decorator, Put);
method_decorator!(post_decorator, Post);
method_decorator!(delete_decorator, Delete);
method_decorator!(head_decorator, Head);
method_decorator!(patch_decorator, Patch);

// TODO: Allow this once Diesel incompatibility is fixed.
// method_decorator!(options_decorator, Options);
