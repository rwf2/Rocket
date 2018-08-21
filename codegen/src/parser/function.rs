use syntax::ast::*;
use syntax::source_map::{Span, Spanned};
use syntax::ext::base::Annotatable;
use utils::{ArgExt, span};

#[derive(Debug)]
pub struct Function(Spanned<(Ident, FnDecl)>);

impl Function {
    pub fn from(annotated: &Annotatable) -> Result<Function, Span> {
        if let Annotatable::Item(ref item) = *annotated {
            if let ItemKind::Fn(ref decl, ..) = item.node {
                let inner = (item.ident, decl.clone().into_inner());
                return Ok(Function(span(inner, item.span)));
            }
        }

        Err(annotated.span())
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
