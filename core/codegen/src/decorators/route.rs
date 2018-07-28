use std::collections::HashSet;
use std::fmt::Display;

use ::{ROUTE_STRUCT_PREFIX, ROUTE_FN_PREFIX, PARAM_PREFIX, URI_INFO_MACRO_PREFIX};
use ::{ROUTE_ATTR, ROUTE_INFO_ATTR};
use parser::{Param, RouteParams};
use utils::*;

use syntax::codemap::{Span, Spanned, dummy_spanned};
use syntax::tokenstream::TokenTree;
use syntax::ast::{Arg, Ident, Item, Stmt, Expr, MetaItem, Path};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::parse::token;
use syntax::symbol::LocalInternedString;
use syntax::ptr::P;

use rocket_http::{Method, MediaType};

fn method_to_path(ecx: &ExtCtxt, method: Method) -> Path {
    quote_enum!(ecx, method => ::rocket_http::Method -> ::rocket::http::Method {
        Options, Get, Post, Put, Delete, Head, Trace, Connect, Patch;
    })
}

// FIXME: I think we also want the attributes here.
fn media_type_to_expr(ecx: &ExtCtxt, ct: Option<MediaType>) -> Option<P<Expr>> {
    ct.map(|ct| {
        let (top, sub) = (ct.top().as_str(), ct.sub().as_str());
        quote_expr!(ecx, ::rocket::http::MediaType {
            source: ::rocket::http::Source::None,
            top: ::rocket::http::Indexed::Concrete(
                ::std::borrow::Cow::Borrowed($top)
            ),
            sub: ::rocket::http::Indexed::Concrete(
                ::std::borrow::Cow::Borrowed($sub)
            ),
            params: ::rocket::http::MediaParams::Static(&[])
        })
    })
}

impl RouteParams {
    fn missing_declared_err<T: Display>(&self, ecx: &ExtCtxt, arg: &Spanned<T>) {
        let (fn_span, fn_name) = (self.annotated_fn.span(), self.annotated_fn.ident());
        ecx.struct_span_err(arg.span, &format!("unused dynamic parameter: `{}`", arg.node))
            .span_note(fn_span, &format!("expected argument named `{}` in `{}` handler",
                                         arg.node, fn_name))
            .emit();
    }

    fn gen_form(&self,
                ecx: &ExtCtxt,
                param: Option<&Spanned<Ident>>,
                form_string: P<Expr>)
                -> Option<Stmt> {
        let arg = param.and_then(|p| self.annotated_fn.find_input(&p.node.name));
        if param.is_none() {
            return None;
        } else if arg.is_none() {
            self.missing_declared_err(ecx, param.unwrap());
            return None;
        }

        let arg = arg.unwrap();
        let name = arg.ident().expect("form param identifier").prepend(PARAM_PREFIX);
        let ty = strip_ty_lifetimes(arg.ty.clone());
        Some(quote_stmt!(ecx,
            #[allow(non_snake_case)]
            let $name: $ty = {
                let mut items = ::rocket::request::FormItems::from($form_string);
                let form = ::rocket::request::FromForm::from_form(items.by_ref(), true);
                #[allow(unreachable_patterns)]
                let obj = match form {
                    Ok(v) => v,
                    Err(_) => return ::rocket::Outcome::Forward(__data)
                };

                if !items.exhaust() {
                    println!("    => The query string {:?} is malformed.", $form_string);
                    return ::rocket::Outcome::Failure(::rocket::http::Status::BadRequest);
                }

                obj
             }
        ).expect("form statement"))
    }

    fn generate_data_statement(&self, ecx: &ExtCtxt) -> Option<Stmt> {
        let param = self.data_param.as_ref().map(|p| &p.value)?;
        let arg = self.annotated_fn.find_input(&param.node.name);
        if arg.is_none() {
            self.missing_declared_err(ecx, param);
            return None;
        }

        let arg = arg.unwrap();
        let name = arg.ident().expect("form param identifier").prepend(PARAM_PREFIX);
        let ty = strip_ty_lifetimes(arg.ty.clone());
        Some(quote_stmt!(ecx,
            #[allow(non_snake_case, unreachable_patterns)]
            let $name: $ty =
                match ::rocket::data::FromData::from_data(__req, __data) {
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
           match __req.uri().query() {
               Some(query) => query,
               None => return ::rocket::Outcome::Forward(__data)
           }
        );

        self.gen_form(ecx, param, expr)
    }

    // TODO: Add some kind of logging facility in Rocket to get be able to log
    // an error/debug message if parsing a parameter fails.
    fn generate_param_statements(&self, ecx: &ExtCtxt) -> Vec<Stmt> {
        let mut fn_param_statements = vec![];
        let params = Param::parse_many(ecx, self.uri.node.path(), self.uri.span.trim(1))
            .unwrap_or_else(|mut diag| { diag.emit(); vec![] });

        // Generate a statement for every declared paramter in the path.
        let mut declared_set = HashSet::new();
        for (i, param) in params.iter().enumerate() {
            declared_set.insert(param.ident().name);
            let ty = match self.annotated_fn.find_input(&param.ident().name) {
                Some(arg) => strip_ty_lifetimes(arg.ty.clone()),
                None => {
                    self.missing_declared_err(ecx, param.inner());
                    continue;
                }
            };

            // Note: the `None` case shouldn't happen if a route is matched.
            let ident = param.ident().prepend(PARAM_PREFIX);
            let expr = match *param {
                Param::Single(_) => quote_expr!(ecx, match __req.get_param_str($i) {
                    Some(s) => <$ty as ::rocket::request::FromParam>::from_param(s),
                    None => return ::rocket::Outcome::Forward(__data)
                }),
                Param::Many(_) => quote_expr!(ecx, match __req.get_raw_segments($i) {
                    Some(s) => <$ty as ::rocket::request::FromSegments>::from_segments(s),
                    None => return ::rocket::Outcome::Forward(__data)
                }),
            };

            let original_ident = param.ident();
            fn_param_statements.push(quote_stmt!(ecx,
                #[allow(non_snake_case, unreachable_patterns)]
                let $ident: $ty = match $expr {
                    Ok(v) => v,
                    Err(e) => {
                        println!("    => Failed to parse '{}': {:?}",
                                 stringify!($original_ident), e);
                        return ::rocket::Outcome::Forward(__data)
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
                ecx.span_err(a.pat.span, "route argument names must be identifiers");
                false
            }
        };

        // Generate the code for `from_request` parameters.
        let all = &self.annotated_fn.decl().inputs;
        for arg in all.iter().filter(from_request) {
            let ident = arg.ident().unwrap().prepend(PARAM_PREFIX);
            let ty = strip_ty_lifetimes(arg.ty.clone());
            fn_param_statements.push(quote_stmt!(ecx,
                #[allow(non_snake_case, unreachable_patterns)]
                let $ident: $ty = match
                        ::rocket::request::FromRequest::from_request(__req) {
                    ::rocket::Outcome::Success(v) => v,
                    ::rocket::Outcome::Forward(_) =>
                        return ::rocket::Outcome::Forward(__data),
                    ::rocket::Outcome::Failure((code, _)) => {
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

    fn generate_uri_macro(&self, ecx: &ExtCtxt) -> P<Item> {
        let macro_args = parse_as_tokens(ecx, "$($token:tt)*");
        let macro_exp = parse_as_tokens(ecx, "$($token)*");
        let macro_name = self.annotated_fn.ident().prepend(URI_INFO_MACRO_PREFIX);

        // What we return if we find an inconsistency throughout.
        let dummy = quote_item!(ecx, pub macro $macro_name($macro_args) { }).unwrap();

        // Hacky check to see if the user's URI was valid.
        if self.uri.span == dummy_spanned(()).span {
            return dummy
        }

        // Extract the route uri path and paramters from the uri.
        let route_path = self.uri.node.to_string();
        let params = match Param::parse_many(ecx, &route_path, self.uri.span.trim(1)) {
            Ok(params) => params,
            Err(mut diag) => {
                diag.cancel();
                return dummy;
            }
        };

        // Generate the list of arguments for the URI.
        let mut fn_uri_args = vec![];
        for param in &params {
            if let Some(arg) = self.annotated_fn.find_input(&param.ident().name) {
                let (pat, ty) = (&arg.pat, &arg.ty);
                fn_uri_args.push(quote_tokens!(ecx, $pat: $ty))
            } else {
                return dummy;
            }
        }

        // Generate the call to the internal URI macro with all the info.
        let args = sep_by_tok(ecx, &fn_uri_args, token::Comma);
        quote_item!(ecx,
            pub macro $macro_name($macro_args) {
                rocket_internal_uri!($route_path, ($args), $macro_exp)
            }
        ).expect("consistent uri macro item")
    }

    fn explode(&self, ecx: &ExtCtxt) -> (LocalInternedString, String, Path, P<Expr>, P<Expr>) {
        let name = self.annotated_fn.ident().name.as_str();
        let path = self.uri.node.to_string();
        let method = method_to_path(ecx, self.method.node);
        let format = self.format.as_ref().map(|kv| kv.value().clone());
        let media_type = option_as_expr(ecx, &media_type_to_expr(ecx, format));
        let rank = option_as_expr(ecx, &self.rank);

        (name, path, method, media_type, rank)
    }
}

// FIXME: Compilation fails when parameters have the same name as the function!
fn generic_route_decorator(known_method: Option<Spanned<Method>>,
                           ecx: &mut ExtCtxt,
                           sp: Span,
                           meta_item: &MetaItem,
                           annotated: Annotatable
                           ) -> Vec<Annotatable> {
    let mut output = Vec::new();

    // Parse the route and generate the code to create the form and param vars.
    let route = RouteParams::from(ecx, sp, known_method, meta_item, &annotated);
    debug!("Route params: {:?}", route);

    let param_statements = route.generate_param_statements(ecx);
    let query_statement = route.generate_query_statement(ecx);
    let data_statement = route.generate_data_statement(ecx);
    let fn_arguments = route.generate_fn_arguments(ecx);
    let uri_macro = route.generate_uri_macro(ecx);

    // Generate and emit the wrapping function with the Rocket handler signature.
    let user_fn_name = route.annotated_fn.ident();
    let route_fn_name = user_fn_name.prepend(ROUTE_FN_PREFIX);
    emit_item(&mut output, quote_item!(ecx,
        // Allow the `unreachable_code` lint for those FromParam impls that have
        // an `Error` associated type of !.
        #[allow(unreachable_code)]
        fn $route_fn_name<'_b>(__req: &'_b ::rocket::Request,  __data: ::rocket::Data)
                -> ::rocket::handler::Outcome<'_b> {
             $param_statements
             $query_statement
             $data_statement
             let responder = $user_fn_name($fn_arguments);
            ::rocket::handler::Outcome::from(__req, responder)
        }
    ).unwrap());

    // Generate and emit the static route info that uses the just generated
    // function as its handler. A proper Rocket route will be created from this.
    let struct_name = user_fn_name.prepend(ROUTE_STRUCT_PREFIX);
    let (name, path, method, media_type, rank) = route.explode(ecx);
    let static_route_info_item =  quote_item!(ecx,
        /// Rocket code generated static route information structure.
        #[allow(non_upper_case_globals)]
        pub static $struct_name: ::rocket::StaticRouteInfo =
            ::rocket::StaticRouteInfo {
                name: $name,
                method: $method,
                path: $path,
                handler: $route_fn_name,
                format: $media_type,
                rank: $rank,
            };
    ).expect("static route info");

    // Attach a `rocket_route_info` attribute to the route info and emit it.
    let attr_name = Ident::from_str(ROUTE_INFO_ATTR);
    let info_attr = quote_attr!(ecx, #[$attr_name]);
    attach_and_emit(&mut output, info_attr, Annotatable::Item(static_route_info_item));

    // Attach a `rocket_route` attribute to the user's function and emit it.
    let attr_name = Ident::from_str(ROUTE_ATTR);
    let route_attr = quote_attr!(ecx, #[$attr_name($struct_name)]);
    attach_and_emit(&mut output, route_attr, annotated);

    // Emit the per-route URI macro.
    emit_item(&mut output, uri_macro);

    output
}

pub fn route_decorator(
    ecx: &mut ExtCtxt, sp: Span, meta_item: &MetaItem, annotated: Annotatable
) -> Vec<Annotatable> {
    generic_route_decorator(None, ecx, sp, meta_item, annotated)
}

macro_rules! method_decorator {
    ($name:ident, $method:ident) => (
        pub fn $name(
            ecx: &mut ExtCtxt, sp: Span, meta_item: &MetaItem, annotated: Annotatable
        ) -> Vec<Annotatable> {
            let i_sp = meta_item.span.shorten_to(stringify!($method).len());
            let method = Some(span(Method::$method, i_sp));
            generic_route_decorator(method, ecx, sp, meta_item, annotated)
        }
    )
}

method_decorator!(get_decorator, Get);
method_decorator!(put_decorator, Put);
method_decorator!(post_decorator, Post);
method_decorator!(delete_decorator, Delete);
method_decorator!(head_decorator, Head);
method_decorator!(patch_decorator, Patch);
method_decorator!(options_decorator, Options);
