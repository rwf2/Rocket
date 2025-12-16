use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use devise::ext::PathExt;
use proc_macro2::{Span, TokenStream};
use syn::{parse::Parser, punctuated::Punctuated};

macro_rules! declare_lints {
    ($($name:ident ( $string:literal) ),* $(,)?) => (
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Lint {
            $($name),*
        }

        impl Lint {
            fn from_str(string: &str) -> Option<Self> {
                $(if string.eq_ignore_ascii_case($string) {
                    return Some(Lint::$name);
                })*

                None
            }

            fn as_str(&self) -> &'static str {
                match self {
                    $(Lint::$name => $string),*
                }
            }

            fn lints() -> &'static str {
                concat!("[" $(,$string,)", "* "]")
            }
        }
    )
}

declare_lints! {
    UnknownFormat("unknown_format"),
    DubiousPayload("dubious_payload"),
    SegmentChars("segment_chars"),
    ArbitraryMain("arbitrary_main"),
    SyncSpawn("sync_spawn"),
}

thread_local! {
    static SUPPRESSIONS: RefCell<HashMap<Lint, HashSet<Range<usize>>>> = RefCell::default();
}

fn span_to_range(span: Span) -> Option<Range<usize>> {
    let string = format!("{span:?}");
    let i = string.find('(')?;
    let j = string[i..].find(')')?;
    let (start, end) = string[(i + 1)..(i + j)].split_once("..")?;
    Some(Range {
        start: start.parse().ok()?,
        end: end.parse().ok()?,
    })
}

impl Lint {
    pub fn suppress_attrs(attrs: &[syn::Attribute], ctxt: Span) {
        let _ = attrs
            .iter()
            .try_for_each(|attr| Lint::suppress_attr(attr, ctxt));
    }

    pub fn suppress_attr(attr: &syn::Attribute, ctxt: Span) -> Result<(), syn::Error> {
        let syn::Meta::List(list) = &attr.meta else {
            return Ok(());
        };

        if list.path.last_ident().is_none_or(|i| i != "suppress") {
            return Ok(());
        }

        Self::suppress_tokens(list.tokens.clone(), ctxt)
    }

    pub fn suppress_tokens(attr_tokens: TokenStream, ctxt: Span) -> Result<(), syn::Error> {
        let lints = Punctuated::<Lint, syn::Token![,]>::parse_terminated.parse2(attr_tokens)?;
        lints.iter().for_each(|lint| lint.suppress(ctxt));
        Ok(())
    }

    pub fn suppress(self, ctxt: Span) {
        SUPPRESSIONS.with_borrow_mut(|s| {
            let range = span_to_range(ctxt).unwrap_or_default();
            s.entry(self).or_default().insert(range);
        })
    }

    pub fn is_suppressed(self, ctxt: Span) -> bool {
        SUPPRESSIONS.with_borrow(|s| {
            let this = span_to_range(ctxt).unwrap_or_default();
            s.get(&self).is_some_and(|set| {
                set.iter()
                    .any(|r| this.start >= r.start && this.end <= r.end)
            })
        })
    }

    pub fn enabled(self, ctxt: Span) -> bool {
        !self.is_suppressed(ctxt)
    }

    pub fn how_to_suppress(self) -> String {
        format!(
            "apply `#[suppress({})]` before the item to suppress this lint",
            self.as_str()
        )
    }
}

impl syn::parse::Parse for Lint {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let name = ident.to_string();
        Lint::from_str(&name).ok_or_else(|| {
            let msg = format!("invalid lint `{name}` (known lints: {})", Lint::lints());
            syn::Error::new(ident.span(), msg)
        })
    }
}
