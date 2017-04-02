mod meta_item_ext;
mod arg_ext;
mod parser_ext;
mod ident_ext;
mod span_ext;

pub use self::arg_ext::ArgExt;
pub use self::meta_item_ext::MetaItemExt;
pub use self::parser_ext::ParserExt;
pub use self::ident_ext::IdentExt;
pub use self::span_ext::SpanExt;

use std::convert::AsRef;

use syntax;
use syntax::parse::token::Token;
use syntax::tokenstream::TokenTree;
use syntax::ast::{Item, Expr};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::codemap::{Span, Spanned, DUMMY_SP};
use syntax::ext::quote::rt::ToTokens;
use syntax::print::pprust::item_to_string;
use syntax::ptr::P;
use syntax::fold::Folder;
use syntax::ast::{Attribute, Lifetime, LifetimeDef, Ty};
use syntax::attr::HasAttrs;

pub fn span<T>(t: T, span: Span) -> Spanned<T> {
    Spanned { node: t, span: span }
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

macro_rules! quote_enum {
    ($ecx:expr, $var:expr => $(::$root:ident)+
     { $($variant:ident),+ ; $($extra:pat => $result:expr),* }) => ({
        use syntax::codemap::DUMMY_SP;
        use syntax::ast::Ident;
        use $(::$root)+::*;
        let root_idents = vec![$(Ident::from_str(stringify!($root))),+];
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

pub struct TyLifetimeRemover;

// FIXME: Doesn't work for T + whatever.
impl Folder for TyLifetimeRemover {
    fn fold_opt_lifetime(&mut self, _: Option<Lifetime>) -> Option<Lifetime> {
        None
    }

    fn fold_lifetime_defs(&mut self, _: Vec<LifetimeDef>) -> Vec<LifetimeDef> {
        vec![]
    }

    fn fold_lifetimes(&mut self, _: Vec<Lifetime>) -> Vec<Lifetime> {
        vec![]
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
