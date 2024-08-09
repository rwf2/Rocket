use transient::{Any, CanRecoverFrom, Co, Downcast};
#[doc(inline)]
pub use transient::{Static, Transient, TypeId};

pub type ErasedError<'r> = Box<dyn Any<Co<'r>> + Send + Sync + 'r>;
pub type ErasedErrorRef<'r> = dyn Any<Co<'r>> + Send + Sync + 'r;

pub fn default_error_type<'r>() -> ErasedError<'r> {
    Box::new(())
}

pub fn downcast<'a, 'r, T: Transient + 'r>(v: &'a ErasedErrorRef<'r>) -> Option<&'a T>
    where T::Transience: CanRecoverFrom<Co<'r>>
{
    v.downcast_ref()
}

/// Upcasts a value to `ErasedError`, falling back to a default if it doesn't implement
/// `Transient`
#[doc(hidden)]
#[macro_export]
macro_rules! resolve_typed_catcher {
    ($T:expr) => ({
        #[allow(unused_imports)]
        use $crate::catcher::resolution::{Resolve, DefaultTypeErase};

        Resolve::new($T).cast()
    });
    () => ({
        $crate::catcher::default_error_type()
    });
}

pub use resolve_typed_catcher;

pub mod resolution {
    use std::marker::PhantomData;

    use transient::{CanTranscendTo, Transient};

    use super::*;

    /// The *magic*.
    ///
    /// `Resolve<T>::item` for `T: Transient` is `<T as Transient>::item`.
    /// `Resolve<T>::item` for `T: !Transient` is `DefaultTypeErase::item`.
    ///
    /// This _must_ be used as `Resolve::<T>:item` for resolution to work. This
    /// is a fun, static dispatch hack for "specialization" that works because
    /// Rust prefers inherent methods over blanket trait impl methods.
    pub struct Resolve<'r, T: 'r>(T, PhantomData<&'r ()>);

    impl<'r, T: 'r> Resolve<'r, T> {
        pub fn new(val: T) -> Self {
            Self(val, PhantomData)
        }
    }

    /// Fallback trait "implementing" `Transient` for all types. This is what
    /// Rust will resolve `Resolve<T>::item` to when `T: !Transient`.
    pub trait DefaultTypeErase<'r>: Sized {
        const SPECIALIZED: bool = false;

        fn cast(self) -> ErasedError<'r> { Box::new(()) }
    }

    impl<'r, T: 'r> DefaultTypeErase<'r> for Resolve<'r, T> {}

    /// "Specialized" "implementation" of `Transient` for `T: Transient`. This is
    /// what Rust will resolve `Resolve<T>::item` to when `T: Transient`.
    impl<'r, T: Transient + Send + Sync + 'r> Resolve<'r, T>
        where T::Transience: CanTranscendTo<Co<'r>>
    {
        pub const SPECIALIZED: bool = true;

        pub fn cast(self) -> ErasedError<'r> { Box::new(self.0) }
    }
}

#[cfg(test)]
mod test {
    // use std::any::TypeId;

    use transient::{Transient, TypeId};

    use super::resolution::{Resolve, DefaultTypeErase};

    struct NotAny;
    #[derive(Transient)]
    struct YesAny;

    #[test]
    fn check_can_determine() {
        let not_any = Resolve::new(NotAny).cast();
        assert_eq!(not_any.type_id(), TypeId::of::<()>());

        let yes_any = Resolve::new(YesAny).cast();
        assert_ne!(yes_any.type_id(), TypeId::of::<()>());
    }

    // struct HasSentinel<T>(T);

    // #[test]
    // fn parent_works() {
    //     let child = resolve!(YesASentinel, HasSentinel<YesASentinel>);
    //     assert!(child.type_name.ends_with("YesASentinel"));
    //     assert_eq!(child.parent.unwrap(), TypeId::of::<HasSentinel<YesASentinel>>());
    //     assert!(child.specialized);

    //     let not_a_direct_sentinel = resolve!(HasSentinel<YesASentinel>);
    //     assert!(not_a_direct_sentinel.type_name.contains("HasSentinel"));
    //     assert!(not_a_direct_sentinel.type_name.contains("YesASentinel"));
    //     assert!(not_a_direct_sentinel.parent.is_none());
    //     assert!(!not_a_direct_sentinel.specialized);
    // }
}
