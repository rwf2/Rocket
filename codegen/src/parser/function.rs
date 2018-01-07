use syntax::ast::*;
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::Annotatable;
use utils::{ArgExt, span};

#[derive(Debug)]
pub struct Function(Spanned<(Ident, FnDecl)>);

impl Function {
    pub fn from(annotated: &Annotatable) -> Result<Function, Span> {
        let inner = match *annotated {
            Annotatable::Item(ref item) => match item.node {
                ItemKind::Fn(ref decl, ..) => {
                    span((item.ident, decl.clone().into_inner()), item.span)
                }
                _ => return Err(item.span)
            },
            Annotatable::TraitItem(ref item) => return Err(item.span),
            Annotatable::ImplItem(ref item) => return Err(item.span),
        };

        Ok(Function(inner))
    }

    pub fn ident(&self) -> &Ident {
        &self.0.node.0
    }

    pub fn decl(&self) -> &FnDecl {
        &self.0.node.1
    }

    pub fn span(&self) -> Span {
        self.0.span
    }

    pub fn find_input<'a>(&'a self, name: &Name) -> Option<&'a Arg> {
        self.decl().inputs.iter().find(|arg| arg.named(name))
    }
}

