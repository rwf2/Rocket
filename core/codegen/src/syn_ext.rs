//! Extensions to `syn` types.

use std::ops::Deref;
use std::hash::{Hash, Hasher};
use std::borrow::Cow;

use syn::{Ident, ext::IdentExt as _, visit::Visit};
use proc_macro2::{Span, TokenStream};
use devise::ext::{PathExt, TypeExt as _};
use rocket_http::ext::IntoOwned;

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

pub trait FnArgExt {
    fn typed(&self) -> Option<(&syn::Ident, &syn::Type)>;
    fn wild(&self) -> Option<&syn::PatWild>;
}

pub trait TypeExt {
    fn unfold_with_ty_macros(&self, names: &[&str], mapper: MacTyMapFn) -> Vec<Child<'_>>;
    fn is_concrete(&self, generic_ident: &[&Ident]) -> bool;
}

pub trait GenericsExt {
    fn type_idents(&self) -> Vec<&Ident>;
}

#[derive(Debug)]
pub struct Child<'a> {
    pub parent: Option<Cow<'a, syn::Type>>,
    pub ty: Cow<'a, syn::Type>,
}

impl Deref for Child<'_> {
    type Target = syn::Type;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

impl IntoOwned for Child<'_> {
    type Owned = Child<'static>;

    fn into_owned(self) -> Self::Owned {
        Child {
            parent: self.parent.into_owned(),
            ty: Cow::Owned(self.ty.into_owned()),
        }
    }
}

type MacTyMapFn = fn(&TokenStream) -> Option<syn::Type>;

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

    /// Create a unique version of the ident `self` based on the hash of `self`,
    /// its span, the current call site, and any additional information provided
    /// by the closure `f`.
    ///
    /// Span::source_file() / line / col are not stable, but the byte span and
    /// some kind of scope identifier do appear in the `Debug` representation
    /// for `Span`. And they seem to be consistent across compilations: "#57
    /// bytes(106..117)" at the time of writing. So we use that.
    fn uniqueify_with<F: FnMut(&mut dyn Hasher)>(&self, mut f: F) -> syn::Ident {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:?}", self.span()).hash(&mut hasher);
        format!("{:?}", Span::call_site()).hash(&mut hasher);
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

fn macro_inner_ty(t: &syn::TypeMacro, names: &[&str], m: MacTyMapFn) -> Option<syn::Type> {
    if !names.iter().any(|k| t.mac.path.last_ident().map_or(false, |i| i == k)) {
        return None;
    }

    let mut ty = m(&t.mac.tokens)?;
    ty.strip_lifetimes();
    Some(ty)
}

impl TypeExt for syn::Type {
    fn unfold_with_ty_macros(&self, names: &[&str], mapper: MacTyMapFn) -> Vec<Child<'_>> {
        struct Visitor<'a, 'm> {
            parents: Vec<Cow<'a, syn::Type>>,
            children: Vec<Child<'a>>,
            names: &'m [&'m str],
            mapper: MacTyMapFn,
        }

        impl<'m> Visitor<'_, 'm> {
            fn new(names: &'m [&'m str], mapper: MacTyMapFn) -> Self {
                Visitor { parents: vec![], children: vec![], names, mapper }
            }
        }

        impl<'a> Visit<'a> for Visitor<'a, '_> {
            fn visit_type(&mut self, ty: &'a syn::Type) {
                let parent = self.parents.last().cloned();

                if let syn::Type::Macro(t) = ty {
                    if let Some(inner_ty) = macro_inner_ty(t, self.names, self.mapper) {
                        let mut visitor = Visitor::new(self.names, self.mapper);
                        if let Some(parent) = parent.clone().into_owned() {
                            visitor.parents.push(parent);
                        }

                        visitor.visit_type(&inner_ty);
                        let mut children = visitor.children.into_owned();
                        self.children.append(&mut children);
                        return;
                    }
                }

                self.children.push(Child { parent, ty: Cow::Borrowed(ty) });
                self.parents.push(Cow::Borrowed(ty));
                syn::visit::visit_type(self, ty);
                self.parents.pop();
            }
        }

        let mut visitor = Visitor::new(names, mapper);
        visitor.visit_type(self);
        visitor.children
    }

    fn is_concrete(&self, generics: &[&Ident]) -> bool {
        struct ConcreteVisitor<'i>(bool, &'i [&'i Ident]);

        impl<'a> Visit<'a> for ConcreteVisitor<'_> {
            fn visit_type(&mut self, ty: &'a syn::Type) {
                use syn::Type::*;

                match ty {
                    Path(t) if self.1.iter().any(|i| t.path.is_ident(*i)) => {
                        self.0 = false;
                    }
                    ImplTrait(_) | Infer(_) | Macro(_) => {
                        self.0 = false;
                    }
                    BareFn(_) | Never(_) => {
                        self.0 = true;
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

impl GenericsExt for syn::Generics {
    fn type_idents(&self) -> Vec<&Ident> {
        self.type_params().map(|p| &p.ident).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_type_unfold_is_generic() {
        use super::TypeExt;

        let ty: syn::Type = syn::parse_quote!(A<B, C<impl Foo>, Box<dyn Foo>, Option<T>>);
        let children = ty.unfold_with_ty_macros(&[], |_| None);
        assert_eq!(children.len(), 8);

        let gen_ident = format_ident!("T");
        let gen = &[&gen_ident];
        assert_eq!(children.iter().filter(|c| c.ty.is_concrete(gen)).count(), 3);
    }
}
