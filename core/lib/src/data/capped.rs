/// Number of bytes read/written and whether that consisted of the entire
/// stream.
#[derive(Debug, Copy, Clone)]
pub struct N {
    /// The number of bytes written out.
    pub written: u64,
    /// Whether the entire stream was read and written out.
    pub complete: bool,
}

impl std::fmt::Display for N {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.written.fmt(f)
    }
}

impl std::ops::Deref for N {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.written
    }
}

/// Encapsulates a value capped to a data limit.
///
/// A `Capped<T>` type represents a `T` that has been limited (capped) to some
/// number of bytes. The internal [`N`] specifies whether the value is complete
/// (also [`Capped::is_complete()`]) or whether it was capped prematurely. A
/// [`Capped`] is returned by various methods of [`DataStream`]. Some
/// `Capped<T>` types, like `Capped<String>` and `Capped<TempFile>`, implement
/// traits like [`FromData`] and [`FromForm`].
///
/// # Example
///
/// Since `Capped<TempFile>` implements `FromData`, it can be used as a data
/// guard. The following Rocket route accepts a raw upload and stores the upload
/// in a different directory depending on whether the file exceeded the data
/// limit or not. See [`TempFile`] for details on temporary file storage
/// locations and limits.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::data::Capped;
/// use rocket::fs::TempFile;
///
/// #[post("/upload", data = "<file>")]
/// async fn upload(mut file: Capped<TempFile<'_>>) -> std::io::Result<()> {
///     if file.is_complete() {
///         file.persist_to("/tmp/complete/file.txt").await?;
///     } else {
///         file.persist_to("/tmp/incomplete/file.txt").await?;
///     }
///
///     Ok(())
/// }
/// ```
///
/// [`DataStream`]: crate::data::DataStream
/// [`FromData`]: crate::data::FromData
/// [`FromForm`]: crate::form::FromForm
/// [`TempFile`]: crate::fs::TempFile
// TODO: `Capped` not particularly usable outside Rocket due to coherence.
#[derive(Debug, Copy, Clone)]
pub struct Capped<T> {
    /// The capped value itself.
    pub value: T,
    /// The number of bytes written and whether `value` is complete.
    pub n: N
}

impl<T> Capped<T> {
    /// Creates a new `Capped` from a `value` and an `n`. Prefer to use
    /// [`Capped::from()`] when possible.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let n = N { written: 2, complete: true };
    /// let capped = Capped::new("hi".to_string(), n);
    /// ```
    #[inline(always)]
    pub fn new(value: T, n: N) -> Self {
        Capped { value, n, }
    }

    /// Creates a new `Capped` from a `value` and the length of `value` `n`,
    /// marking `value` as complete. Prefer to use [`Capped::from()`] when
    /// possible.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let string = "hi";
    /// let capped = Capped::complete("hi", string.len());
    /// ```
    #[inline(always)]
    pub fn complete(value: T, len: usize) -> Self {
        Capped { value, n: N { written: len as u64, complete: true } }
    }

    /// Converts a `Capped<T>` to `Capped<U>` by applying `f` to the contained
    /// value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let n = N { written: 2, complete: true };
    /// let capped: Capped<usize> = Capped::new(10usize, n);
    /// let mapped: Capped<String> = capped.map(|n| n.to_string());
    /// ```
    #[inline(always)]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Capped<U> {
        Capped { value: f(self.value), n: self.n }
    }

    /// Returns `true` if `self.n.written` is `0`, that is, no bytes were
    /// written to `value`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let n = N { written: 2, complete: true };
    /// let capped = Capped::new("hi".to_string(), n);
    /// assert!(!capped.is_empty());
    ///
    /// let n = N { written: 0, complete: true };
    /// let capped = Capped::new("".to_string(), n);
    /// assert!(capped.is_empty());
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.n.written == 0
    }

    /// Returns `true` if `self.n.complete`, that is, `value` represents the
    /// entire data stream.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let n = N { written: 2, complete: true };
    /// let capped = Capped::new("hi".to_string(), n);
    /// assert!(capped.is_complete());
    ///
    /// let n = N { written: 4, complete: false };
    /// let capped = Capped::new("hell".to_string(), n);
    /// assert!(!capped.is_complete());
    /// ```
    #[inline(always)]
    pub fn is_complete(&self) -> bool {
        self.n.complete
    }

    /// Returns the internal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Capped, N};
    ///
    /// let n = N { written: 2, complete: true };
    /// let capped = Capped::new("hi".to_string(), n);
    /// assert_eq!(capped.into_inner(), "hi");
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> std::ops::Deref for Capped<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for Capped<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: AsRef<[u8]>> From<T> for Capped<T> {
    /// Creates a `Capped<T>` from a `value`, setting `complete` to `true`.
    fn from(value: T) -> Self {
        let len = value.as_ref().len();
        Capped::complete(value, len)
    }
}

use crate::response::{self, Responder};
use crate::request::Request;

impl<'r, 'o: 'r, T: Responder<'r, 'o>> Responder<'r, 'o> for Capped<T> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        self.value.respond_to(request)
    }
}

macro_rules! impl_strict_from_form_field_from_capped {
    ($T:ty) => (const _: () = {
        use $crate::form::{FromFormField, ValueField, DataField, Result};
        use $crate::data::Capped;

        #[crate::async_trait]
        impl<'v> FromFormField<'v> for $T {
            fn default() -> Option<Self> {
                <Capped<$T> as FromFormField<'v>>::default().map(|c| c.value)
            }

            fn from_value(f: ValueField<'v>) -> Result<'v, Self> {
                let capped = <Capped<$T> as FromFormField<'v>>::from_value(f)?;
                if !capped.is_complete() {
                    Err((None, Some(capped.n.written)))?;
                }

                Ok(capped.value)
            }

            async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
                let capped = <Capped<$T> as FromFormField<'v>>::from_data(field);
                let capped = capped.await?;
                if !capped.is_complete() {
                    Err((None, Some(capped.n.written)))?;
                }

                Ok(capped.value)
            }
        }
    };)
}

macro_rules! impl_strict_from_data_from_capped {
    ($T:ty) => (
        #[crate::async_trait]
        impl<'r> $crate::data::FromData<'r> for $T {
            type Error = <$crate::data::Capped<Self> as $crate::data::FromData<'r>>::Error;

            async fn from_data(
                r: &'r $crate::Request<'_>,
                d: $crate::Data<'r>
            ) -> $crate::data::Outcome<'r, Self> {
                use $crate::outcome::Outcome::*;
                use std::io::{Error, ErrorKind::UnexpectedEof};

                match <$crate::data::Capped<$T> as FromData>::from_data(r, d).await {
                    Success(p) if p.is_complete() => Success(p.into_inner()),
                    Success(_) => {
                        let e = Error::new(UnexpectedEof, "data limit exceeded");
                        Error((Status::BadRequest, e.into()))
                    },
                    Forward(d) => Forward(d),
                    Error((s, e)) => Error((s, e)),
                }
            }
        }
    )
}
