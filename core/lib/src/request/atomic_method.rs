use crate::http::Method;

pub struct AtomicMethod(ref_swap::RefSwap<'static, Method>);

#[inline(always)]
const fn makeref(method: Method) -> &'static Method {
    match method {
        Method::Get => &Method::Get,
        Method::Put => &Method::Put,
        Method::Post => &Method::Post,
        Method::Delete => &Method::Delete,
        Method::Options => &Method::Options,
        Method::Head => &Method::Head,
        Method::Trace => &Method::Trace,
        Method::Connect => &Method::Connect,
        Method::Patch => &Method::Patch,
    }
}

impl AtomicMethod {
    #[inline]
    pub fn new(value: Method) -> Self {
        Self(ref_swap::RefSwap::new(makeref(value)))
    }

    #[inline]
    pub fn load(&self) -> Method {
        *self.0.load(std::sync::atomic::Ordering::Acquire)
    }

    #[inline]
    pub fn store(&self, new: Method) {
        self.0.store(makeref(new), std::sync::atomic::Ordering::Release)
    }
}
