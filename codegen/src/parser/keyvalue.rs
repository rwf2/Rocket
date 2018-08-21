use syntax::source_map::{Spanned, Span, dummy_spanned, DUMMY_SP};
use syntax::ext::base::ExtCtxt;
use syntax::tokenstream::TokenTree;
use syntax::ext::quote::rt::ToTokens;
use utils::span;

/// A spanned key-value pair in an attribute.
#[derive(Debug, Clone)]
pub struct KVSpanned<V> {
    /// Span for the full key/value pair.
    pub span: Span,
    /// The spanned key.
    pub key: Spanned<String>,
    /// The spanned value.
    pub value: Spanned<V>
}

impl<V> KVSpanned<V> {
    /// Maps a reference of the inner value of this span using `f`. The key,
    /// value, and full spans will remain the same in the new KVSpan.
    pub fn map_ref<U, F: FnOnce(&V) -> U>(&self, f: F) -> KVSpanned<U> {
        KVSpanned {
            span: self.span,
            key: self.key.clone(),
            value: span(f(&self.value.node), self.value.span)
        }
    }

    /// Returns the unspanned key. Purely for convenience.
    pub fn key(&self) -> &String {
        &self.key.node
    }

    /// Returns the unspanned value. Purely for convenience.
    pub fn value(&self) -> &V {
        &self.value.node
    }
}

impl<T: Default> Default for KVSpanned<T> {
    /// Returns a dummy KVSpan with an empty key for the default value of T.
    fn default() -> KVSpanned<T> {
        dummy_kvspanned("", T::default())
    }
}

impl<T: ToTokens> ToTokens for KVSpanned<T> {
    fn to_tokens(&self, cx: &ExtCtxt) -> Vec<TokenTree> {
        self.value().to_tokens(cx)
    }
}

/// Creates a KVSpanned value with dummy (meaningless) spans.
pub fn dummy_kvspanned<T, S: ToString>(key: S, value: T) -> KVSpanned<T> {
    KVSpanned {
        span: DUMMY_SP,
        key: dummy_spanned(key.to_string()),
        value: dummy_spanned(value),
    }
}
