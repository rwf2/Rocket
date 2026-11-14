use std::sync::atomic::{AtomicU8, Ordering};

use crate::http::Method;

pub struct AtomicMethod(AtomicU8);

const METHODS: [Method; 9] = [
    Method::Get,
    Method::Put,
    Method::Post,
    Method::Delete,
    Method::Options,
    Method::Head,
    Method::Trace,
    Method::Connect,
    Method::Patch,
];

#[inline(always)]
const fn index(method: Method) -> u8 {
    match method {
        Method::Get => 0,
        Method::Put => 1,
        Method::Post => 2,
        Method::Delete => 3,
        Method::Options => 4,
        Method::Head => 5,
        Method::Trace => 6,
        Method::Connect => 7,
        Method::Patch => 8,
    }
}

impl AtomicMethod {
    #[inline]
    pub fn new(value: Method) -> Self {
        Self(AtomicU8::new(index(value)))
    }

    #[inline]
    pub fn load(&self) -> Method {
        METHODS[self.0.load(Ordering::Acquire) as usize]
    }

    #[inline]
    pub fn store(&self, new: Method) {
        self.0.store(index(new), Ordering::Release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for method in METHODS {
            assert_eq!(METHODS[index(method) as usize], method);

            let atomic = AtomicMethod::new(method);
            assert_eq!(atomic.load(), method);

            let atomic = AtomicMethod::new(Method::Get);
            atomic.store(method);
            assert_eq!(atomic.load(), method);
        }
    }
}
