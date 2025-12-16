//! Extension traits implemented by several HTTP types.

use std::borrow::Cow;

use state::InitCell;

/// Trait implemented by types that can be converted into owned versions of
/// themselves.
pub trait IntoOwned {
    /// The owned version of the type.
    type Owned: 'static;

    /// Converts `self` into an owned version of itself.
    fn into_owned(self) -> Self::Owned;
}

impl<T: IntoOwned> IntoOwned for Option<T> {
    type Owned = Option<T::Owned>;

    #[inline(always)]
    fn into_owned(self) -> Self::Owned {
        self.map(|inner| inner.into_owned())
    }
}

impl<T: IntoOwned> IntoOwned for Vec<T> {
    type Owned = Vec<T::Owned>;

    #[inline(always)]
    fn into_owned(self) -> Self::Owned {
        self.into_iter().map(|inner| inner.into_owned()).collect()
    }
}

impl<T: IntoOwned + Send + Sync> IntoOwned for InitCell<T>
where
    T::Owned: Send + Sync,
{
    type Owned = InitCell<T::Owned>;

    #[inline(always)]
    fn into_owned(self) -> Self::Owned {
        self.map(|inner| inner.into_owned())
    }
}

impl<A: IntoOwned, B: IntoOwned> IntoOwned for (A, B) {
    type Owned = (A::Owned, B::Owned);

    #[inline(always)]
    fn into_owned(self) -> Self::Owned {
        (self.0.into_owned(), self.1.into_owned())
    }
}

impl<B: 'static + ToOwned + ?Sized> IntoOwned for Cow<'_, B> {
    type Owned = Cow<'static, B>;

    #[inline(always)]
    fn into_owned(self) -> <Self as IntoOwned>::Owned {
        Cow::Owned(self.into_owned())
    }
}

macro_rules! impl_into_owned_self {
    ($($T:ty),*) => ($(
        impl IntoOwned for $T {
            type Owned = Self;

            #[inline(always)]
            fn into_owned(self) -> <Self as IntoOwned>::Owned {
                self
            }
        }
    )*)
}

impl_into_owned_self!(bool);
impl_into_owned_self!(u8, u16, u32, u64, usize);
impl_into_owned_self!(i8, i16, i32, i64, isize);

use std::path::Path;

// Outside of http, this is used by a test.
#[doc(hidden)]
pub trait Normalize {
    fn normalized_str(&self) -> Cow<'_, str>;
}

impl<T: AsRef<Path>> Normalize for T {
    #[cfg(windows)]
    fn normalized_str(&self) -> Cow<'_, str> {
        self.as_ref().to_string_lossy().replace('\\', "/").into()
    }

    #[cfg(not(windows))]
    fn normalized_str(&self) -> Cow<'_, str> {
        self.as_ref().to_string_lossy()
    }
}
