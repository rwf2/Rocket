mod meta_item_ext;
mod arg_ext;
mod parser_ext;
mod ident_ext;
mod span_ext;
mod expr_ext;
mod generics_ext;

pub use self::arg_ext::ArgExt;
pub use self::meta_item_ext::MetaItemExt;
pub use self::parser_ext::ParserExt;
pub use self::ident_ext::IdentExt;
pub use self::span_ext::SpanExt;
pub use self::expr_ext::ExprExt;
pub use self::generics_ext::GenericParamExt;

use std::convert::AsRef;

use syntax;
use syntax::parse::token::Token;
use syntax::tokenstream::TokenTree;
use syntax::ast::{Item, Expr, Attribute, Ty};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::codemap::{Span, Spanned, DUMMY_SP};
use syntax::ext::quote::rt::ToTokens;
use syntax::print::pprust::item_to_string;
use syntax::symbol::{Ident, Symbol};
use syntax::fold::Folder;
use syntax::attr::HasAttrs;
use syntax::ptr::P;

macro_rules! debug {
    ($($t:tt)*) => (
        // Enable debug logs if the DEBUG_ENV_VAR is set.
        if ::std::env::var(::DEBUG_ENV_VAR).is_ok() {
            eprintln!("--> {}:{} ({})", file!(), line!(), module_path!());
            eprintln!($($t)*);
            eprintln!();
        }
    )
}

pub fn span<T>(t: T, span: Span) -> Spanned<T> {
    Spanned { node: t, span }
}

pub fn sep_by_tok<T>(ecx: &ExtCtxt, things: &[T], token: Token) -> Vec<TokenTree>
    where T: ToTokens
{
    let mut output: Vec<TokenTree> = vec![];
    for (i, thing) in things.iter().enumerate() {
        output.extend(thing.to_tokens(ecx));
        if i < things.len() - 1 {
            output.push(TokenTree::Token(DUMMY_SP, token.clone()));
        }
    }

    output
}

pub fn option_as_expr<T: ToTokens>(ecx: &ExtCtxt, opt: &Option<T>) -> P<Expr> {
    match *opt {
        Some(ref item) => quote_expr!(ecx, Some($item)),
        None => quote_expr!(ecx, None),
    }
}

pub fn emit_item(items: &mut Vec<Annotatable>, item: P<Item>) {
    debug!("Emitting item:\n{}", item_to_string(&item));
    items.push(Annotatable::Item(item));
}

pub fn attach_and_emit(out: &mut Vec<Annotatable>, attr: Attribute, to: Annotatable) {
    syntax::attr::mark_used(&attr);
    syntax::attr::mark_known(&attr);

    // Attach the attribute to the user's function and emit it.
    if let Annotatable::Item(user_item) = to {
        let item = user_item.map_attrs(|mut attrs| {
            attrs.push(attr);
            attrs
        });

        emit_item(out, item);
    }
}

pub fn parse_as_tokens(ecx: &ExtCtxt, s: &str) -> Vec<TokenTree> {
    use syntax_pos::FileName;
    use syntax::parse::parse_stream_from_source_str as parse_stream;

    parse_stream(FileName::ProcMacroSourceCode, s.into(), ecx.parse_sess, None)
        .into_trees()
        .collect()
}

pub struct TyLifetimeRemover;

impl Folder for TyLifetimeRemover {
    fn fold_ident(&mut self, ident: Ident) -> Ident {
        if ident.as_str().starts_with('\'') {
            Ident::new(Symbol::intern("'_"), ident.span)
        } else {
            ident
        }
    }
}

pub fn strip_ty_lifetimes(ty: P<Ty>) -> P<Ty> {
    TyLifetimeRemover.fold_ty(ty)
}

// Lifted from Rust's lexer, except this takes a `char`, not an `Option<char>`.
fn ident_start(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' ||
    (c > '\x7f' && c.is_xid_start())
}

// Lifted from Rust's lexer, except this takes a `char`, not an `Option<char>`.
fn ident_continue(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
    c == '_' || (c > '\x7f' && c.is_xid_continue())
}

pub fn is_valid_ident<S: AsRef<str>>(s: S) -> bool {
    let string = s.as_ref();
    if string.is_empty() {
        return false;
    }

    for (i, c) in string.chars().enumerate() {
        if i == 0 {
            if !ident_start(c) {
                return false;
            }
        } else if !ident_continue(c) {
            return false;
        }
    }

    true
}

pub fn split_idents(path: &str) -> Vec<Ident> {
    path.split("::").map(|segment| Ident::from_str(segment)).collect()
}

macro_rules! quote_enum {
    ($ecx:expr, $var:expr => $(::$from_root:ident)+ -> $(::$to_root:ident)+
     { $($variant:ident),+ ; $($extra:pat => $result:expr),* }) => ({
        use syntax::codemap::DUMMY_SP;
        use syntax::ast::Ident;
        use $(::$from_root)+::*;
        let root_idents = vec![$(Ident::from_str(stringify!($to_root))),+];
        match $var {
            $($variant => {
                let variant = Ident::from_str(stringify!($variant));
                let mut idents = root_idents.clone();
                idents.push(variant);
                $ecx.path_global(DUMMY_SP, idents)
            })+
            $($extra => $result),*
        }
    })
}

macro_rules! try_parse {
    ($sp:expr, $parse:expr) => (
        match $parse {
            Ok(v) => v,
            Err(mut e) => { e.emit(); return DummyResult::expr($sp); }
        }
    )
}

macro_rules! p {
    ("parameter", $num:expr) => (
        if $num == 1 { "parameter" } else { "parameters" }
    );

    ($num:expr, "was") => (
        if $num == 1 { "1 was".into() } else { format!("{} were", $num) }
    );

    ($num:expr, "parameter") => (
        if $num == 1 { "1 parameter".into() } else { format!("{} parameters", $num) }
    )
}
