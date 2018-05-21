use proc_macro::Span;

use quote::ToTokens;

pub trait Spanned {
    fn span(&self) -> Span;
}

// FIXME: Remove this once proc_macro's stabilize.
impl<T: ToTokens> Spanned for T {
    fn span(&self) -> Span {
        let token_stream = self.into_token_stream();
        let mut iter = token_stream.into_iter();
        let mut span = match iter.next() {
            Some(tt) => tt.span().unstable(),
            None => {
                return Span::call_site();
            }
        };

        for tt in iter {
            if let Some(joined) = span.join(tt.span().unstable()) {
                span = joined;
            }
        }

        span
    }
}
