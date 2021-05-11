//! Extensions to `syn` types.

use std::ops::Deref;
use std::hash::{Hash, Hasher};

use crate::syn::{self, Ident, ext::IdentExt as _, visit::Visit};
use crate::proc_macro2::Span;

pub trait IdentExt {
    fn prepend(&self, string: &str) -> syn::Ident;
    fn append(&self, string: &str) -> syn::Ident;
    fn with_span(self, span: Span) -> syn::Ident;
    fn rocketized(&self) -> syn::Ident;
    fn uniqueify_with<F: FnMut(&mut dyn Hasher)>(&self, f: F) -> syn::Ident;
}

pub trait ReturnTypeExt {
    fn ty(&self) -> Option<&syn::Type>;
}

pub trait TokenStreamExt {
    fn respanned(&self, span: crate::proc_macro2::Span) -> Self;
}

pub trait FnArgExt {
    fn typed(&self) -> Option<(&syn::Ident, &syn::Type)>;
    fn wild(&self) -> Option<&syn::PatWild>;
}

#[derive(Debug)]
pub struct Child<'a> {
    pub parent: Option<&'a syn::Type>,
    pub ty: &'a syn::Type,
}

impl Deref for Child<'_> {
    type Target = syn::Type;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

pub trait TypeExt {
    fn unfold(&self) -> Vec<Child<'_>>;
    fn is_concrete(&self, generic_ident: &[&Ident]) -> bool;
}

impl IdentExt for syn::Ident {
    fn prepend(&self, string: &str) -> syn::Ident {
        syn::Ident::new(&format!("{}{}", string, self.unraw()), self.span())
    }

    fn append(&self, string: &str) -> syn::Ident {
        syn::Ident::new(&format!("{}{}", self, string), self.span())
    }

    fn with_span(mut self, span: Span) -> syn::Ident {
        self.set_span(span);
        self
    }

    fn rocketized(&self) -> syn::Ident {
        self.prepend(crate::ROCKET_IDENT_PREFIX)
    }

    fn uniqueify_with<F: FnMut(&mut dyn Hasher)>(&self, mut f: F) -> syn::Ident {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::collections::hash_map::DefaultHasher;

        // Keep a global counter (+ thread ID later) to generate unique ids.
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        COUNTER.fetch_add(1, Ordering::AcqRel).hash(&mut hasher);
        f(&mut hasher);

        self.append(&format!("_{}", hasher.finish()))
    }
}

impl ReturnTypeExt for syn::ReturnType {
    fn ty(&self) -> Option<&syn::Type> {
        match self {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(ty),
        }
    }
}

impl TokenStreamExt for crate::proc_macro2::TokenStream {
    fn respanned(&self, span: crate::proc_macro2::Span) -> Self {
        self.clone().into_iter().map(|mut token| {
            token.set_span(span);
            token
        }).collect()
    }
}

impl FnArgExt for syn::FnArg {
    fn typed(&self) -> Option<(&Ident, &syn::Type)> {
        match self {
            syn::FnArg::Typed(arg) => match *arg.pat {
                syn::Pat::Ident(ref pat) => Some((&pat.ident, &arg.ty)),
                _ => None
            }
            _ => None,
        }
    }

    fn wild(&self) -> Option<&syn::PatWild> {
        match self {
            syn::FnArg::Typed(arg) => match *arg.pat {
                syn::Pat::Wild(ref pat) => Some(pat),
                _ => None
            }
            _ => None,
        }
    }
}

impl TypeExt for syn::Type {
    fn unfold(&self) -> Vec<Child<'_>> {
        #[derive(Default)]
        struct Visitor<'a> {
            parents: Vec<&'a syn::Type>,
            children: Vec<Child<'a>>,
        }

        impl<'a> Visit<'a> for Visitor<'a> {
            fn visit_type(&mut self, ty: &'a syn::Type) {
                self.children.push(Child { parent: self.parents.last().cloned(), ty });
                self.parents.push(ty);
                syn::visit::visit_type(self, ty);
                self.parents.pop();
            }
        }

        let mut visitor = Visitor::default();
        visitor.visit_type(self);
        visitor.children
    }

    fn is_concrete(&self, generics: &[&Ident]) -> bool {
        struct ConcreteVisitor<'i>(bool, &'i [&'i Ident]);

        impl<'a, 'i> Visit<'a> for ConcreteVisitor<'i> {
            fn visit_type(&mut self, ty: &'a syn::Type) {
                use syn::Type::*;

                match ty {
                    Path(t) if self.1.iter().any(|i| t.path.is_ident(*i)) => {
                        self.0 = false;
                        return;
                    }
                    ImplTrait(_) | Infer(_) | Macro(_) => {
                        self.0 = false;
                        return;
                    }
                    BareFn(_) | Never(_) => {
                        self.0 = true;
                        return;
                    },
                    _ => syn::visit::visit_type(self, ty),
                }
            }
        }

        let mut visitor = ConcreteVisitor(true, generics);
        visitor.visit_type(self);
        visitor.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_type_unfold_is_generic() {
        use super::{TypeExt, syn};

        let ty: syn::Type = syn::parse_quote!(A<B, C<impl Foo>, Box<dyn Foo>, Option<T>>);
        let children = ty.unfold();
        assert_eq!(children.len(), 8);

        let gen_ident = format_ident!("T");
        let gen = &[&gen_ident];
        assert_eq!(children.iter().filter(|c| c.ty.is_concrete(gen)).count(), 3);
    }
}
