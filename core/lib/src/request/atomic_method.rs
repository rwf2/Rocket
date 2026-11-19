use std::sync::atomic::{AtomicU8, Ordering};

use crate::http::Method;

pub struct AtomicMethod(AtomicU8);

impl AtomicMethod {
    #[inline]
    pub fn new(value: Method) -> Self {
        Self(AtomicU8::new(value as u8))
    }

    #[inline]
    pub fn load(&self) -> Method {
        Method::ALL_VARIANTS[self.0.load(Ordering::Acquire) as usize]
    }

    #[inline]
    pub fn set(&mut self, new: Method) {
        *self = Self::new(new);
    }

    #[inline]
    pub fn store(&self, new: Method) {
        self.0.store(new as u8, Ordering::Release)
    }
}

impl Clone for AtomicMethod {
    fn clone(&self) -> Self {
        Self(AtomicU8::new(self.0.load(Ordering::Acquire)))
    }
}
