//! Extension traits implemented by several HTTP types.

use smallvec::{Array, SmallVec};
use state::Storage;

// TODO: It would be nice if we could somehow have one trait that could give us
// either SmallVec or Vec.
/// Trait implemented by types that can be converted into a collection.
pub trait IntoCollection<T>: Sized {
    /// Converts `self` into a collection.
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A>;

    #[doc(hidden)]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, f: F) -> SmallVec<A>;

    #[doc(hidden)]
    fn mapped_vec<U, F: FnMut(T) -> U>(self, f: F) -> Vec<U> {
        let small = self.mapped::<U, F, [U; 0]>(f);
        small.into_vec()
    }
}

impl<T> IntoCollection<T> for T {
    #[inline]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        let mut vec = SmallVec::new();
        vec.push(self);
        vec
    }

    #[inline(always)]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        f(self).into_collection()
    }
}

impl<T> IntoCollection<T> for Vec<T> {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        SmallVec::from_vec(self)
    }

    #[inline]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, f: F) -> SmallVec<A> {
        self.into_iter().map(f).collect()
    }
}

impl<T: Clone> IntoCollection<T> for &[T] {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        self.iter().cloned().collect()
    }

    #[inline]
    fn mapped<U, F, A: Array<Item=U>>(self, f: F) -> SmallVec<A>
        where F: FnMut(T) -> U
    {
        self.iter().cloned().map(f).collect()
    }
}

impl<T, const N: usize> IntoCollection<T> for [T; N] {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        std::array::IntoIter::new(self).collect()
    }

    #[inline]
    fn mapped<U, F, A: Array<Item=U>>(self, f: F) -> SmallVec<A>
        where F: FnMut(T) -> U
    {
        std::array::IntoIter::new(self).map(f).collect()
    }
}

use std::borrow::Cow;

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
        self.into_iter()
            .map(|inner| inner.into_owned())
            .collect()
    }
}

impl<T: IntoOwned + Send + Sync> IntoOwned for Storage<T>
    where T::Owned: Send + Sync
{
    type Owned = Storage<T::Owned>;

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
