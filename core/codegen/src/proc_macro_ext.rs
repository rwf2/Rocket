use std::ops::{Bound, RangeBounds};

use proc_macro::{Span, Diagnostic, /* MultiSpan */};
use syntax_pos::{Span as InnerSpan, Pos, BytePos};

pub type PResult<T> = ::std::result::Result<T, Diagnostic>;

pub type DResult<T> = ::std::result::Result<T, Diagnostics>;

// An experiment.
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics(vec![])
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.0.push(diag);
    }

    pub fn join(mut self, mut diags: Diagnostics) -> Self {
        self.0.append(&mut diags.0);
        self
    }

    pub fn emit_head(self) -> Diagnostic {
        let mut iter = self.0.into_iter();
        let mut last = iter.next().expect("Diagnostic::emit_head empty");
        for diag in iter {
            last.emit();
            last = diag;
        }

        last
    }

    pub fn head_err_or<T>(self, ok: T) -> PResult<T> {
        match self.0.is_empty() {
            true => Ok(ok),
            false => Err(self.emit_head())
        }
    }

    pub fn err_or<T>(self, ok: T) -> DResult<T> {
        match self.0.is_empty() {
            true => Ok(ok),
            false => Err(self)
        }
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diag: Diagnostic) -> Self {
        Diagnostics(vec![diag])
    }
}

impl From<Vec<Diagnostic>> for Diagnostics {
    fn from(diags: Vec<Diagnostic>) -> Self {
        Diagnostics(diags)
    }
}

pub trait SpanExt {
    fn subspan<R: RangeBounds<usize>>(self, range: R) -> Option<Span>;
}

impl SpanExt for Span {
    /// Create a `subspan` from `start` to `end`.
    fn subspan<R: RangeBounds<usize>>(self, range: R) -> Option<Span> {
        let inner: InnerSpan = unsafe { ::std::mem::transmute(self) };
        let length = inner.hi().to_usize() - inner.lo().to_usize();

        let start = match range.start_bound() {
            Bound::Included(&lo) => lo,
            Bound::Excluded(&lo) => lo + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&hi) => hi + 1,
            Bound::Excluded(&hi) => hi,
            Bound::Unbounded => length,
        };

        // Bounds check the values, preventing addition overflow and OOB spans.
        if start > u32::max_value() as usize
            || end > u32::max_value() as usize
            || (u32::max_value() - start as u32) < inner.lo().to_u32()
            || (u32::max_value() - end as u32) < inner.lo().to_u32()
            || start >= end
            || end > length
        {
            return None;
        }

        let new_lo = inner.lo() + BytePos(start as u32);
        let new_hi = inner.lo() + BytePos(end as u32);
        let new_inner = inner.with_lo(new_lo).with_hi(new_hi);
        Some(unsafe { ::std::mem::transmute(new_inner) })
    }
}
