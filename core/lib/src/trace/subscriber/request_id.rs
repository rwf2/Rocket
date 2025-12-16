use std::cell::Cell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::thread::ThreadId;

use tracing::span::{Attributes, Id};
use tracing::Subscriber;
use tracing_subscriber::registry::{LookupSpan, SpanRef};
use tracing_subscriber::{layer::Context, Layer};

pub struct RequestIdLayer;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct RequestId(u128);

#[derive(Default)]
pub struct IdentHasher(u128);

impl RequestId {
    fn new() -> Self {
        thread_local! {
            pub static COUNTER: Cell<u64> = const { Cell::new(0) };
            pub static THREAD_ID: Cell<Option<ThreadId>> = const { Cell::new(None) };
        }

        let thread_id = THREAD_ID.get().unwrap_or_else(|| {
            let id = std::thread::current().id();
            THREAD_ID.set(Some(id));
            id
        });

        let local_id = COUNTER.get();
        COUNTER.set(local_id.wrapping_add(1));

        let mut hasher = IdentHasher::default();
        thread_id.hash(&mut hasher);
        local_id.hash(&mut hasher);
        RequestId(hasher.0)
    }

    pub fn of<R: for<'a> LookupSpan<'a>>(span: &SpanRef<'_, R>) -> Option<Self> {
        span.extensions().get::<Self>().copied()
    }

    pub fn current() -> Option<Self> {
        RequestIdLayer::current()
    }

    fn short(&self) -> u32 {
        let mut x = ((self.0 & (0xFFFFFFFF << 48)) >> 48) as u32;
        x = (x ^ (x >> 16)).wrapping_mul(0x21f0aaad);
        x = (x ^ (x >> 15)).wrapping_mul(0x735a2d97);
        x = x ^ (x >> 15);
        x
    }

    pub fn layer() -> RequestIdLayer {
        RequestIdLayer
    }
}

impl RequestIdLayer {
    thread_local! {
        static CURRENT_REQUEST_ID: Cell<Option<RequestId>> = const { Cell::new(None) };
    }

    pub fn current() -> Option<RequestId> {
        Self::CURRENT_REQUEST_ID.get()
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for RequestIdLayer {
    fn on_new_span(&self, _: &Attributes<'_>, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("new_span: span does not exist");
        if span.name() == "request" {
            span.extensions_mut().replace(RequestId::new());
        }
    }

    fn on_enter(&self, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("enter: span does not exist");
        if span.name() == "request" {
            Self::CURRENT_REQUEST_ID.set(RequestId::of(&span));
        }
    }

    fn on_exit(&self, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("enter: span does not exist");
        if span.name() == "request" {
            Self::CURRENT_REQUEST_ID.set(None);
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.short(), f)
    }
}

impl fmt::LowerHex for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.short(), f)
    }
}

impl fmt::UpperHex for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.short(), f)
    }
}

impl Hasher for IdentHasher {
    fn finish(&self) -> u64 {
        self.0 as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 << 8) | (byte as u128);
        }
    }

    fn write_u64(&mut self, i: u64) {
        // https://github.com/skeeto/hash-prospector
        fn shuffle(mut x: u64) -> u64 {
            x = x.wrapping_add(1);
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
            x = x ^ (x >> 31);
            x
        }

        self.0 = (self.0 << 64) | shuffle(i) as u128;
    }
}
