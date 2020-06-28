//! A structure representing a response from dispatching a local request.
//!
//! This structure is a thin wrapper around [`Response`]. It implements no
//! methods of its own; all functionality is exposed via the [`Deref`] and
//! [`DerefMut`] implementations with a target of `Response`. In other words,
//! when invoking methods, a `LocalResponse` can be treated exactly as if it
//! were a `Response`.

macro_rules! impl_response {
    ($import:literal $(@$prefix:tt $suffix:tt)? $name:ident) =>
{
    impl<'c> $name<'c> {
        /// Consumes `self` reads its body into a string. If `self` doesn't have
        /// a body, reading fails, or string conversion (for non-UTF-8 bodies)
        /// fails, returns `None`.
        ///
        /// # Example
        ///
        /// ```rust,ignore
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let response = client.get("/").body("Hello!").dispatch();
        /// assert_eq!(response.into_string().unwrap(), "Hello!");
        /// # })
        /// ```
        #[inline(always)]
        pub $($prefix)? fn into_string(self) -> Option<String> {
            self._into_string() $(.$suffix)?
        }

        /// Consumes `self` and reads its body into a `Vec` of `u8` bytes. If
        /// `self` doesn't have a body or reading fails returns `None`. Note
        /// that `self`'s `body` is consumed after a call to this method.
        ///
        /// # Example
        ///
        /// ```rust,ignore
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let response = client.get("/").body("Hello!").dispatch();
        /// assert_eq!(response.into_bytes().unwrap(), "Hello!".as_bytes());
        /// # })
        /// ```
        #[inline(always)]
        pub $($prefix)? fn into_bytes(self) -> Option<Vec<u8>> {
            self._into_bytes() $(.$suffix)?
        }
    }

    impl std::fmt::Debug for LocalResponse<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self._response().fmt(f)
        }
    }

    impl<'c> std::ops::Deref for LocalResponse<'c> {
        type Target = Response<'c>;

        fn deref(&self) -> &Response<'c> {
            self._response()
        }
    }
}}
