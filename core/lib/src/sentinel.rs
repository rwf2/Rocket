use std::fmt;
use std::any::TypeId;

use crate::{Rocket, Ignite};

/// An automatic last line of defense against launching an invalid [`Rocket`].
///
/// A sentinel, automatically run on [`ignition`](Rocket::ignite()), can trigger
/// a launch abort should an instance fail to meet arbitrary conditions. Every
/// type that appears in a **mounted** route's type signature is eligible to be
/// a sentinel. Of these, those that implement `Sentinel` have their
/// [`abort()`](Sentinel::abort()) method invoked automatically, immediately
/// after ignition, once for each unique type. Sentinels inspect the finalized
/// instance of `Rocket` and can trigger a launch abort by returning `true`.
///
/// # Built-In Sentinels
///
/// The [`State<T>`] type is a sentinel that triggers an abort if the finalized
/// `Rocket` instance is not managing state for type `T`. Doing so prevents
/// run-time failures of the `State` request guard.
///
/// [`State<T>`]: crate::State
/// [`State`]: crate::State
///
/// ## Example
///
/// As an example, consider the following simple application:
///
/// ```rust
/// # use rocket::*;
/// # type Response = ();
/// #[get("/<id>")]
/// fn index(id: usize, state: &State<String>) -> Response {
///     /* ... */
/// }
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/", routes![index])
/// }
///
/// # use rocket::{Config, error::ErrorKind};
/// # rocket::async_test(async {
/// #    let result = rocket().configure(Config::debug_default()).ignite().await;
/// #    assert!(matches!(result.unwrap_err().kind(), ErrorKind::SentinelAborts(..)));
/// # })
/// ```
///
/// At ignition time, effected by the `#[launch]` attribute here, Rocket probes
/// all types in all mounted routes for `Sentinel` implementations. In this
/// example, the types are: `usize`, `State<String>`, and `Response`. Those that
/// implement `Sentinel` are queried for an abort trigger via their
/// [`Sentinel::abort()`] method. In this example, the sentinel types are
/// [`State`] and _potentially_ `Response`, if it implements
/// `Sentinel`. If `abort()` returns true, launch is aborted with a
/// corresponding error.
///
/// In this example, launch will be aborted because state of type `String` is
/// not being managed. To correct the error and allow launching to proceed
/// nominally, a value of type `String` must be managed:
///
/// ```rust
/// # use rocket::*;
/// # type Response = ();
/// # #[get("/<id>")]
/// # fn index(id: usize, state: &State<String>) -> Response {
/// #     /* ... */
/// # }
/// #
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build()
///         .mount("/", routes![index])
///         .manage(String::from("my managed string"))
/// }
///
/// # use rocket::{Config, error::ErrorKind};
/// # rocket::async_test(async {
/// #    rocket().configure(Config::debug_default()).ignite().await.unwrap();
/// # })
/// ```
///
/// # Embedded Sentinels
///
/// Embedded types -- type parameters of already eligible types -- are also
/// eligible to be sentinels. Consider the following route:
///
/// ```rust
/// # use rocket::*;
/// # use either::Either;
/// # type Inner<T> = Option<T>;
/// # type Foo = ();
/// # type Bar = ();
/// #[get("/")]
/// fn f(guard: Option<&State<String>>) -> Either<Foo, Inner<Bar>> {
///     unimplemented!()
/// }
/// ```
///
/// The directly eligible sentinel types, guard and responders, are:
///
///   * `Option<&State<String>>`
///   * `Either<Foo, Inner<Bar>>`
///
/// In addition, all embedded types are _also_ eligible. These are:
///
///   * `&State<String>`
///   * `State<String>`
///   * `String`
///   * `Foo`
///   * `Inner<Bar>`
///   * `Bar`
///
/// A type, whether embedded or not, is queried if it is a `Sentinel` _and_ none
/// of its parent types are sentinels. Said a different way, if every _directly_
/// eligible type is viewed as the root of an acyclic graph with edges between a
/// type and its type parameters, the _first_ `Sentinel` in breadth-first order
/// is queried:
///
/// ```text
/// 1.     Option<&State<String>>        Either<Foo, Inner<Bar>>
///                 |                           /         \
/// 2.        &State<String>                   Foo     Inner<Bar>
///                 |                                     |
/// 3.         State<String>                              Bar
///                 |
/// 4.            String
/// ```
///
/// In each graph above, types are queried from top to bottom, level 1 to 4.
/// Querying continues down paths where the parents were _not_ sentinels. For
/// example, if `Option` is a sentinel but `Either` is not, then querying stops
/// for the left subgraph (`Option`) but continues for the right subgraph
/// `Either`.
///
/// # Limitations
///
/// Because Rocket must know which `Sentinel` implementation to query based on
/// its _written_ type, generally only explicitly written, resolved, concrete
/// types are eligible to be sentinels. A typical application will only work
/// with such types, but there are several common cases to be aware of.
///
/// ## `impl Trait`
///
/// Occasionally an existential `impl Trait` may find its way into return types:
///
/// ```rust
/// # use rocket::*;
/// # use either::Either;
/// use rocket::response::Responder;
/// # type AnotherSentinel = ();
///
/// #[get("/")]
/// fn f<'r>() -> Either<impl Responder<'r, 'static>, AnotherSentinel> {
///     /* ... */
///     # Either::Left(())
/// }
/// ```
///
/// **Note:** _Rocket actively discourages using `impl Trait` in route
/// signatures. In addition to impeding sentinel discovery, doing so decreases
/// the ability to gleam a handler's functionality based on its type signature._
///
/// The return type of the route `f` depends on its implementation. At present,
/// it is not possible to name the underlying concrete type of an `impl Trait`
/// at compile-time and thus not possible to determine if it implements
/// `Sentinel`. As such, existentials _are not_ eligible to be sentinels.
///
/// That being said, this limitation only applies _per embedding_: types
/// embedded inside of an `impl Trait` _are_ eligible. As such, in the example
/// above, the named `AnotherSentinel` type continues to be eligible.
///
/// When possible, prefer to name all types:
///
/// ```rust
/// # use rocket::*;
/// # use either::Either;
/// # type AbortingSentinel = ();
/// # type AnotherSentinel = ();
/// #[get("/")]
/// fn f() -> Either<AbortingSentinel, AnotherSentinel> {
///     /* ... */
///     # unimplemented!()
/// }
/// ```
///
/// ## Aliases
///
/// _Embedded_ sentinels made opaque by a type alias will fail to be considered;
/// the aliased type itself _is_ considered. In the example below, only
/// `Result<Foo, Bar>` will be considered, while the embedded `Foo` and `Bar`
/// will not.
///
/// ```rust
/// # use rocket::get;
/// # type Foo = ();
/// # type Bar = ();
/// type SomeAlias = Result<Foo, Bar>;
///
/// #[get("/")]
/// fn f() -> SomeAlias {
///     /* ... */
///     # unimplemented!()
/// }
/// ```
///
/// Note, however, that `Option<T>` and [`Debug<T>`](crate::response::Debug) are
/// a sentinels if `T: Sentinel`, and `Result<T, E>` and `Either<T, E>` are
/// sentinels if _both_ `T: Sentinel, E: Sentinel`. Thus, for these specific
/// cases, a type alias _will_ "consider" embeddings. Nevertheless, prefer to
/// write concrete types when possible.
///
/// ## Type Macros
///
/// It is impossible to determine, a priori, what a type macro will expand to.
/// As such, Rocket is unable to determine which sentinels, if any, a type macro
/// references, and thus no sentinels are discovered from type macros.
///
/// Even approximations are impossible. For example, consider the following:
///
/// ```rust
/// # use rocket::*;
/// macro_rules! MyType {
///     (State<'_, u32>) => (&'_ rocket::Config)
/// }
///
/// #[get("/")]
/// fn f(guard: MyType![State<'_, u32>]) {
///     /* ... */
/// }
/// ```
///
/// While the `MyType![State<'_, u32>]` type _appears_ to contain a `State`
/// sentinel, the macro actually expands to `&'_ rocket::Config`, which is _not_
/// the `State` sentinel.
///
/// Because Rocket knows the exact syntax expected by type macros that it
/// exports, such as the [typed stream] macros, discovery in these macros works
/// as expected. You should prefer not to use type macros aside from those
/// exported by Rocket, or if necessary, restrict your use to those that always
/// expand to types without sentinels.
///
/// [typed stream]: crate::response::stream
///
/// # Custom Sentinels
///
/// Any type can implement `Sentinel`, and the implementation can arbitrarily
/// inspect an ignited instance of `Rocket`. For illustration, consider the
/// following implementation of `Sentinel` for a custom `Responder` which
/// requires:
///
///   * state for a type `T` to be managed
///   * a catcher for status code `400` at base `/`
///
/// ```rust
/// use rocket::{Rocket, Ignite, Sentinel};
/// # struct MyResponder;
/// # struct T;
///
/// impl Sentinel for MyResponder {
///     fn abort(rocket: &Rocket<Ignite>) -> bool {
///         if rocket.state::<T>().is_none() {
///             return true;
///         }
///
///         if !rocket.catchers().any(|c| c.code == Some(400) && c.base == "/") {
///             return true;
///         }
///
///         false
///     }
/// }
/// ```
///
/// If a `MyResponder` is returned by any mounted route, its `abort()` method
/// will be invoked. If the required conditions aren't met, signaled by
/// returning `true` from `abort()`, Rocket aborts launch.
pub trait Sentinel {
    /// Returns `true` if launch should be aborted and `false` otherwise.
    fn abort(rocket: &Rocket<Ignite>) -> bool;
}

impl<T: Sentinel> Sentinel for Option<T> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        T::abort(rocket)
    }
}

// In the next impls, we want to run _both_ sentinels _without_ short
// circuiting, for the logs. Ideally we could check if these are the same type
// or not, but `TypeId` only works with `'static`, and adding those bounds to
// `T` and `E` would reduce the types for which the implementations work, which
// would mean more types that we miss in type applies. When the type _isn't_ an
// alias, however, the existence of these implementations is strictly worse.

impl<T: Sentinel, E: Sentinel> Sentinel for Result<T, E> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        let left = T::abort(rocket);
        let right = E::abort(rocket);
        left || right
    }
}

impl<T: Sentinel, E: Sentinel> Sentinel for either::Either<T, E> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        let left = T::abort(rocket);
        let right = E::abort(rocket);
        left || right
    }
}

/// A sentinel that never aborts. The `Responder` impl for `Debug` will never be
/// called, so it's okay to not abort for failing `T: Sentinel`.
impl<T> Sentinel for crate::response::Debug<T> {
    fn abort(_: &Rocket<Ignite>) -> bool {
        false
    }
}

/// The information resolved from a `T: ?Sentinel` by the `resolve!()` macro.
#[derive(Clone, Copy)]
pub struct Sentry {
    /// The type ID of `T`.
    pub type_id: TypeId,
    /// The type name `T` as a string.
    pub type_name: &'static str,
    /// The type ID of type in which `T` is nested if not a top-level type.
    pub parent: Option<TypeId>,
    /// The source (file, column, line) location of the resolved `T`.
    pub location: (&'static str, u32, u32),
    /// The value of `<T as Sentinel>::SPECIALIZED` or the fallback.
    ///
    /// This is `true` when `T: Sentinel` and `false` when `T: !Sentinel`.
    pub specialized: bool,
    /// The value of `<T as Sentinel>::abort` or the fallback.
    pub abort: fn(&Rocket<Ignite>) -> bool,
}

/// Query `sentinels`, once for each unique `type_id`, returning an `Err` of all
/// of the sentinels that triggered an abort or `Ok(())` if none did.
pub(crate) fn query<'s>(
    sentinels: impl Iterator<Item = &'s Sentry>,
    rocket: &Rocket<Ignite>,
) -> Result<(), Vec<Sentry>> {
    use std::collections::{HashMap, VecDeque};

    // Build a graph of the sentinels.
    let mut roots: VecDeque<&'s Sentry> = VecDeque::new();
    let mut map: HashMap<TypeId, VecDeque<&'s Sentry>> = HashMap::new();
    for sentinel in sentinels {
        match sentinel.parent {
            Some(parent) => map.entry(parent).or_default().push_back(sentinel),
            None => roots.push_back(sentinel),
        }
    }

    // Traverse the graph in breadth-first order. If we find a specialized
    // sentinel, query it (once for a unique type) and don't traverse its
    // children. Otherwise, traverse its children. Record queried aborts.
    let mut remaining = roots;
    let mut visited: HashMap<TypeId, bool> = HashMap::new();
    let mut aborted = vec![];
    while let Some(sentinel) = remaining.pop_front() {
        if sentinel.specialized {
            if *visited.entry(sentinel.type_id).or_insert_with(|| (sentinel.abort)(rocket)) {
                aborted.push(sentinel);
            }
        } else if let Some(mut children) = map.remove(&sentinel.type_id) {
            remaining.append(&mut children);
        }
    }

    match aborted.is_empty() {
        true => Ok(()),
        false => Err(aborted.into_iter().cloned().collect())
    }
}

impl fmt::Debug for Sentry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sentry")
            .field("type_id", &self.type_id)
            .field("type_name", &self.type_name)
            .field("parent", &self.parent)
            .field("location", &self.location)
            .field("default", &self.specialized)
            .finish()
    }
}

/// Resolves a `T` to the specialized or fallback implementation of
/// `Sentinel`, returning a `Sentry` struct with the resolved items.
#[doc(hidden)]
#[macro_export]
macro_rules! resolve {
    ($T:ty $(, $P:ty)?) => ({
        #[allow(unused_imports)]
        use $crate::sentinel::resolution::{Resolve, DefaultSentinel as _};

        $crate::sentinel::Sentry {
            type_id: std::any::TypeId::of::<$T>(),
            type_name: std::any::type_name::<$T>(),
            parent: None $(.or(Some(std::any::TypeId::of::<$P>())))?,
            location: (std::file!(), std::line!(), std::column!()),
            specialized: Resolve::<$T>::SPECIALIZED,
            abort: Resolve::<$T>::abort,
        }
    })
}

pub use resolve;

pub mod resolution {
    use super::*;

    /// The *magic*.
    ///
    /// `Resolve<T>::item` for `T: Sentinel` is `<T as Sentinel>::item`.
    /// `Resolve<T>::item` for `T: !Sentinel` is `DefaultSentinel::item`.
    ///
    /// This _must_ be used as `Resolve::<T>:item` for resolution to work. This
    /// is a fun, static dispatch hack for "specialization" that works because
    /// Rust prefers inherent methods over blanket trait impl methods.
    pub struct Resolve<T: ?Sized>(std::marker::PhantomData<T>);

    /// Fallback trait "implementing" `Sentinel` for all types. This is what
    /// Rust will resolve `Resolve<T>::item` to when `T: !Sentinel`.
    pub trait DefaultSentinel {
        const SPECIALIZED: bool = false;

        fn abort(_: &Rocket<Ignite>) -> bool { false }
    }

    impl<T: ?Sized> DefaultSentinel for T {}

    /// "Specialized" "implementation" of `Sentinel` for `T: Sentinel`. This is
    /// what Rust will resolve `Resolve<T>::item` to when `T: Sentinel`.
    impl<T: Sentinel + ?Sized> Resolve<T> {
        pub const SPECIALIZED: bool = true;

        pub fn abort(rocket: &Rocket<Ignite>) -> bool {
            T::abort(rocket)
        }
    }
}

#[cfg(test)]
mod test {
    use std::any::TypeId;
    use crate::sentinel::resolve;

    struct NotASentinel;
    struct YesASentinel;

    impl super::Sentinel for YesASentinel {
        fn abort(_: &crate::Rocket<crate::Ignite>) -> bool {
            unimplemented!()
        }
    }

    #[test]
    fn check_can_determine() {
        let not_a_sentinel = resolve!(NotASentinel);
        assert!(not_a_sentinel.type_name.ends_with("NotASentinel"));
        assert!(!not_a_sentinel.specialized);

        let yes_a_sentinel = resolve!(YesASentinel);
        assert!(yes_a_sentinel.type_name.ends_with("YesASentinel"));
        assert!(yes_a_sentinel.specialized);
    }

    struct HasSentinel<T>(T);

    #[test]
    fn parent_works() {
        let child = resolve!(YesASentinel, HasSentinel<YesASentinel>);
        assert!(child.type_name.ends_with("YesASentinel"));
        assert_eq!(child.parent.unwrap(), TypeId::of::<HasSentinel<YesASentinel>>());
        assert!(child.specialized);

        let not_a_direct_sentinel = resolve!(HasSentinel<YesASentinel>);
        assert!(not_a_direct_sentinel.type_name.contains("HasSentinel"));
        assert!(not_a_direct_sentinel.type_name.contains("YesASentinel"));
        assert!(not_a_direct_sentinel.parent.is_none());
        assert!(!not_a_direct_sentinel.specialized);
    }
}
