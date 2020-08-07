macro_rules! getter_method {
    ($doc_prelude:literal, $desc:literal, $f:ident -> $r:ty) => (
        getter_method!(@$doc_prelude, $f, $desc, $r,
            concat!("let ", stringify!($f), " = response.", stringify!($f), "();"));
    );
    (@$doc_prelude:literal, $f:ident, $desc:expr, $r:ty, $use_it:expr) => (
        /// Returns the
        #[doc = $desc]
        /// of `self`.
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = $doc_prelude]
        ///
        /// # Client::_test(|_, _, response| {
        /// let response: LocalResponse = response;
        #[doc = $use_it]
        /// # });
        /// ```
        #[inline(always)]
        pub fn $f(&self) -> $r {
            self._response().$f()
        }
    )
}

macro_rules! pub_response_impl {
    ($doc_prelude:literal $($prefix:tt $suffix:tt)?) =>
{
    getter_method!($doc_prelude, "HTTP status",
        status -> crate::http::Status);

    getter_method!($doc_prelude, "Content-Type, if a valid one is set,",
        content_type -> Option<crate::http::ContentType>);

    getter_method!($doc_prelude, "HTTP headers",
        headers -> &crate::http::HeaderMap<'_>);

    getter_method!($doc_prelude, "HTTP cookies as set in the `Set-Cookie` header",
        cookies -> Vec<crate::http::Cookie<'_>>);

    getter_method!($doc_prelude, "response body, if there is one,",
        body -> Option<&crate::response::ResponseBody<'_>>);

    /// Consumes `self` and reads the entirety of its body into a string. If
    /// `self` doesn't have a body, reading fails, or string conversion (for
    /// non-UTF-8 bodies) fails, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let string = response.into_string();
    /// # });
    /// ```
    #[inline(always)]
    pub $($prefix)? fn into_string(self) -> Option<String> {
        self._into_string() $(.$suffix)?
    }

    /// Consumes `self` and reads the entirety of its body into a `Vec` of `u8`
    /// bytes. If `self` doesn't have a body or reading fails, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let bytes = response.into_bytes();
    /// # });
    /// ```
    #[inline(always)]
    pub $($prefix)? fn into_bytes(self) -> Option<Vec<u8>> {
        self._into_bytes() $(.$suffix)?
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn _ensure_impls_exist() {
        fn is_debug<T: std::fmt::Debug>() {}
        is_debug::<Self>();
    }
}}
