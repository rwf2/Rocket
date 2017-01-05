use syntax::ast::{LitKind, NestedMetaItem, MetaItemKind, Lit};
use syntax::symbol::Symbol;

pub trait MetaItemExt {
    fn name_value(&self) -> Option<(&Symbol, &Lit)>;
    fn str_lit(&self) -> Option<&Symbol>;
    fn int_lit(&self) -> Option<u128>;
}

impl MetaItemExt for NestedMetaItem {
    fn name_value(&self) -> Option<(&Symbol, &Lit)> {
        self.meta_item().and_then(|mi| match mi.node {
            MetaItemKind::NameValue(ref l) => Some((&mi.name, l)),
            _ => None,
        })
    }

    fn str_lit(&self) -> Option<&Symbol> {
        self.literal().and_then(|lit| match lit.node {
            LitKind::Str(ref s, _) => Some(s),
            _ => None,
        })
    }

    fn int_lit(&self) -> Option<u128> {
        self.literal().and_then(|lit| match lit.node {
            LitKind::Int(n, _) => Some(n),
            _ => None,
        })
    }
}
