use crate::http::Method;

pub struct AtomicMethod(ref_swap::RefSwap<'static, Method>);

impl AtomicMethod {
    #[inline]
    pub fn new(value: Method) -> Self {
        Self(ref_swap::RefSwap::new(value.as_ref()))
    }

    #[inline]
    pub fn load(&self) -> Method {
        *self.0.load(std::sync::atomic::Ordering::Acquire)
    }

    #[inline]
    pub fn set(&mut self, new: Method) {
        *self = Self::new(new);
    }

    #[inline]
    pub fn store(&self, new: Method) {
        self.0
            .store(new.as_ref(), std::sync::atomic::Ordering::Release)
    }
}

impl Clone for AtomicMethod {
    fn clone(&self) -> Self {
        let inner = self.0.load(std::sync::atomic::Ordering::Acquire);
        Self(ref_swap::RefSwap::new(inner))
    }
}
