mod parse;

use std::ops::Deref;
use std::hash::Hash;

use crate::name::Name;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Parameter {
    Static(Name),
    Ignored(Dynamic),
    Dynamic(Dynamic),
    Guard(Guard),
}

#[derive(Debug, Clone)]
pub struct Dynamic {
    pub name: Name,
    pub index: usize,
    pub trailing: bool,
}

#[derive(Debug, Clone)]
pub struct Guard {
    pub source: Dynamic,
    pub fn_ident: syn::Ident,
    pub ty: syn::Type,
}

impl Parameter {
    pub fn r#static(&self) -> Option<&Name> {
        match self {
            Parameter::Static(s) => Some(s),
            _ => None
        }
    }

    pub fn ignored(&self) -> Option<&Dynamic> {
        match self {
            Parameter::Ignored(d) => Some(d),
            _ => None
        }
    }

    pub fn take_dynamic(self) -> Option<Dynamic> {
        match self {
            Parameter::Dynamic(d) => Some(d),
            Parameter::Guard(g) => Some(g.source),
            _ => None
        }
    }

    pub fn dynamic(&self) -> Option<&Dynamic> {
        match self {
            Parameter::Dynamic(d) => Some(d),
            Parameter::Guard(g) => Some(&g.source),
            _ => None
        }
    }

    pub fn dynamic_mut(&mut self) -> Option<&mut Dynamic> {
        match self {
            Parameter::Dynamic(d) => Some(d),
            Parameter::Guard(g) => Some(&mut g.source),
            _ => None
        }
    }

    pub fn guard(&self) -> Option<&Guard> {
        match self {
            Parameter::Guard(g) => Some(g),
            _ => None
        }
    }
}

impl Dynamic {
    // This isn't public since this `Dynamic` should always be an `Ignored`.
    pub fn is_wild(&self) -> bool {
        &self.name == "_"
    }
}

impl Guard {
    pub fn from(source: Dynamic, fn_ident: syn::Ident, ty: syn::Type) -> Self {
        Guard { source, fn_ident, ty }
    }
}

macro_rules! impl_derived {
    ($T:ty => $U:ty = $v:ident) => (
        impl Deref for $T {
            type Target = $U;

            fn deref(&self) -> &Self::Target {
                &self.$v
            }
        }

        impl PartialEq for $T {
            fn eq(&self, other: &Self) -> bool {
                self.$v == other.$v
            }
        }

        impl Eq for $T {  }

        impl Hash for $T {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.$v.hash(state)
            }
        }
    )
}

impl_derived!(Dynamic => Name = name);
impl_derived!(Guard => Dynamic = source);
