use std::ops::{Index, RangeFrom, RangeTo};
use std::cell::UnsafeCell;

use parking_lot::{RawMutex, lock_api::RawMutex as _};

mod private {
    /// Sealed trait for types that can be shared in a `SharedStack`.
    ///
    /// The type of values passed to
    /// [`local_cache`](crate::request::local_cache) must implement this trait.
    /// Since this trait is sealed, the types implementing this trait are known
    /// and finite: `String` and `Vec<T> for all T: Sync + Send + 'static`.
    // UNSAFE: Needs to have a stable address when deref'd.
    pub unsafe trait Shareable: std::ops::Deref + Sync + Send + 'static {
        /// The current length of the owned shareable.
        fn len(&self) -> usize;
    }

    unsafe impl Shareable for String {
        fn len(&self) -> usize { self.len() }
    }

    unsafe impl<T: Send + Sync + 'static> Shareable for Vec<T> {
        fn len(&self) -> usize { self.len() }
    }
}

pub use private::Shareable;

/// A stack of strings (chars of bytes) that can be shared between threads while
/// remaining internally mutable and while allowing references into the stack to
/// persist across mutations.
pub struct SharedStack<T: Shareable> {
    stack: UnsafeCell<Vec<T>>,
    mutex: RawMutex,
}

impl<T: Shareable> SharedStack<T>
    where T::Target: Index<RangeFrom<usize>, Output = T::Target> +
                     Index<RangeTo<usize>, Output = T::Target>
{
    /// Creates a new stack.
    pub fn new() -> Self {
        SharedStack {
            stack: UnsafeCell::new(vec![]),
            mutex: RawMutex::INIT,
        }
    }

    /// Pushes the string `S` onto the stack. Returns a reference of the string
    /// in the stack.
    pub(crate) fn push<S: Into<T>>(&self, string: S) -> &T::Target {
        // SAFETY:
        //   * Aliasing: We retrieve a mutable reference to the last slot (via
        //     `push()`) and then return said reference as immutable; these
        //     occur in serial, so they don't alias. This method accesses a
        //     unique slot each call: the last slot, subsequently replaced by
        //     `push()` each next call. No other method accesses the internal
        //     buffer directly. Thus, the outstanding reference to the last slot
        //     is never accessed again mutably, preserving aliasing guarantees.
        //   * Liveness: The returned reference is to a `String`; we must ensure
        //     that the `String` is never dropped while `self` lives. This is
        //     guaranteed by returning a reference with the same lifetime as
        //     `self`, so `self` can't be dropped while the string is live, and
        //     by never removing elements from the internal `Vec` thus not
        //     dropping `String` itself: `push()` is the only mutating operation
        //     called on `Vec`, which preserves all previous elements; the
        //     stability of `String` itself means that the returned address
        //     remains valid even after internal realloc of `Vec`.
        //   * Thread-Safety: Parallel calls to `push_one` without exclusion
        //     would result in a race to `vec.push()`; `RawMutex` ensures that
        //     this doesn't occur.
        unsafe {
            self.mutex.lock();
            let vec: &mut Vec<T> = &mut *self.stack.get();
            vec.push(string.into());
            let last = vec.last().expect("push() => non-empty");
            self.mutex.unlock();
            last
        }
    }

    /// Just like `push` but `string` must already be the owned `T`.
    pub fn push_owned(&self, string: T) -> &T::Target {
        self.push(string)
    }

    /// Pushes the string `S` onto the stack which is assumed to internally
    /// contain two strings with the first string being of length `n`. Returns
    /// references to the two strings on the stack.
    ///
    /// # Panics
    ///
    /// Panics if `string.len() < len`.
    pub(crate) fn push_split<S: Into<T>>(&self, string: S, n: usize) -> (&T::Target, &T::Target) {
        let buffered = self.push(string);
        let a = &buffered[..n];
        let b = &buffered[n..];
        (a, b)
    }

    /// Pushes the strings `a` and `b` onto the stack without allocating for
    /// both strings. Returns references to the two strings on the stack.
    pub(crate) fn push_two<'a, V>(&'a self, a: V, b: V) -> (&'a T::Target, &'a T::Target)
        where T: From<V> + Extend<V>,
    {
        let mut value = T::from(a);
        let split_len = value.len();
        value.extend(Some(b));
        self.push_split(value, split_len)
    }
}

unsafe impl<T: Shareable> Sync for SharedStack<T> {}
