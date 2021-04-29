//! Traits and structs related to automagically generating documentation for your Rocket routes

use std::{collections::HashMap, marker::PhantomData};

use rocket_http::ContentType;

mod has_schema;

#[derive(Default)]
pub struct Docs(HashMap<ContentType, DocContent>);

#[derive(Default)]
pub struct DocContent {
    title: Option<String>,
    description: Option<String>,
    content_type: Option<String>,
}


pub trait Documented {
    fn docs() -> Docs;
}

pub struct Schema {
    pub type_name: &'static str,
    pub documented: bool,
    pub docs: Docs,
}

#[doc(hidden)]
#[macro_export]
macro_rules! resolve_doc { 
    ($T:ty) => ({
        #[allow(unused_imports)]
        use $crate::doc::resolution::{Resolve, Undocumented as _};

        $crate::doc::Schema {
            type_name: std::any::type_name::<$T>(),
            documented: Resolve::<$T>::DOCUMENTED,
            docs: Resolve::<$T>::docs(),
        }
    })
}

pub use resolve_doc;

pub mod resolution {
    use super::*;

    pub struct Resolve<T: ?Sized>(PhantomData<T>);

    pub trait Undocumented {
        const DOCUMENTED: bool = false;

        fn docs() -> Docs {
            Docs::default()
        }
    }

    impl<T: ?Sized> Undocumented for T { }

    impl<T: Documented + ?Sized> Resolve<T> {
        pub const DOCUMENTED: bool = true;
    
        pub fn docs() -> Docs {
            T::docs()
        }
    }
}
