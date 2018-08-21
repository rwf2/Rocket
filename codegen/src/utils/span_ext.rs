use syntax::source_map::{Span, BytePos};

pub trait SpanExt {
    /// Trim the span on the left and right by `length`.
    fn trim(self, length: u32) -> Span;

    /// Trim the span on the left by `length`.
    fn trim_left(self, length: usize) -> Span;

    /// Trim the span on the right by `length`.
    fn trim_right(self, length: usize) -> Span;

    // Trim from the right so that the span is `length` in size.
    fn shorten_to(self, to_length: usize) -> Span;

    // Trim from the left so that the span is `length` in size.
    fn shorten_upto(self, length: usize) -> Span;
}

impl SpanExt for Span {
    fn trim_left(self, length: usize) -> Span {
        self.with_lo(self.lo() + BytePos(length as u32))
    }

    fn trim_right(self, length: usize) -> Span {
        self.with_hi(self.hi() - BytePos(length as u32))
    }

    fn shorten_to(self, to_length: usize) -> Span {
        self.with_hi(self.lo() + BytePos(to_length as u32))
    }

    fn shorten_upto(self, length: usize) -> Span {
        self.with_lo(self.hi() - BytePos(length as u32))
    }

    fn trim(self, length: u32) -> Span {
        self.with_lo(self.lo() + BytePos(length))
            .with_hi(self.hi() - BytePos(length))
    }
}
