use smallvec::{Array, SmallVec};

// TODO: It would be nice if we could somehow have one trait that could give us
// either SmallVec or Vec.
pub trait IntoCollection<T> {
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A>;
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, f: F) -> SmallVec<A>;
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

macro_rules! impl_for_slice {
    ($($size:tt)*) => (
        impl<'a, T: Clone> IntoCollection<T> for &'a [T $($size)*] {
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
    )
}

impl_for_slice!();
impl_for_slice!(; 1);
impl_for_slice!(; 2);
impl_for_slice!(; 3);
impl_for_slice!(; 4);
impl_for_slice!(; 5);
impl_for_slice!(; 6);
impl_for_slice!(; 7);
impl_for_slice!(; 8);
impl_for_slice!(; 9);
impl_for_slice!(; 10);
