use std::fmt;
use std::ops::Deref;
use std::any::type_name;

use ref_cast::RefCast;
use yansi::Paint;

use crate::{Phase, Rocket, Ignite, Sentinel};
use crate::request::{self, FromRequest, Request};
use crate::outcome::Outcome;
use crate::http::Status;

/// Request guard to retrieve managed state.
///
/// A reference `&State<T>` type is a request guard which retrieves the managed
/// state managing for some type `T`. A value for the given type must previously
/// have been registered to be managed by Rocket via [`Rocket::manage()`]. The
/// type being managed must be thread safe and sendable across thread
/// boundaries as multiple handlers in multiple threads may be accessing the
/// value at once. In other words, it must implement [`Send`] + [`Sync`] +
/// `'static`.
///
/// # Example
///
/// Imagine you have some configuration struct of the type `MyConfig` that you'd
/// like to initialize at start-up and later access it in several handlers. The
/// following example does just this:
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// use rocket::State;
///
/// // In a real application, this would likely be more complex.
/// struct MyConfig {
///     user_val: String
/// }
///
/// #[get("/")]
/// fn index(state: &State<MyConfig>) -> String {
///     format!("The config value is: {}", state.user_val)
/// }
///
/// #[get("/raw")]
/// fn raw_config_value(state: &State<MyConfig>) -> &str {
///     &state.user_val
/// }
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build()
///         .mount("/", routes![index, raw_config_value])
///         .manage(MyConfig { user_val: "user input".to_string() })
/// }
/// ```
///
/// # Within Request Guards
///
/// Because `State` is itself a request guard, managed state can be retrieved
/// from another request guard's implementation using either
/// [`Request::guard()`] or [`Rocket::state()`]. In the following code example,
/// the `Item` request guard retrieves `MyConfig` from managed state:
///
/// ```rust
/// use rocket::State;
/// use rocket::request::{self, Request, FromRequest};
/// use rocket::outcome::IntoOutcome;
/// use rocket::http::Status;
///
/// # struct MyConfig { user_val: String };
/// struct Item<'r>(&'r str);
///
/// #[rocket::async_trait]
/// impl<'r> FromRequest<'r> for Item<'r> {
///     type Error = ();
///
///     async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
///         // Using `State` as a request guard. Use `inner()` to get an `'r`.
///         let outcome = request.guard::<&State<MyConfig>>().await
///             .map(|my_config| Item(&my_config.user_val));
///
///         // Or alternatively, using `Rocket::state()`:
///         let outcome = request.rocket().state::<MyConfig>()
///             .map(|my_config| Item(&my_config.user_val))
///             .or_forward(Status::InternalServerError);
///
///         outcome
///     }
/// }
/// ```
///
/// # Testing with `State`
///
/// When unit testing your application, you may find it necessary to manually
/// construct a type of `State` to pass to your functions. To do so, use the
/// [`State::get()`] static method or the `From<&T>` implementation:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::State;
///
/// struct MyManagedState(usize);
///
/// #[get("/")]
/// fn handler(state: &State<MyManagedState>) -> String {
///     state.0.to_string()
/// }
///
/// let mut rocket = rocket::build().manage(MyManagedState(127));
/// let state = State::get(&rocket).expect("managed `MyManagedState`");
/// assert_eq!(handler(state), "127");
///
/// let managed = MyManagedState(77);
/// assert_eq!(handler(State::from(&managed)), "77");
/// ```
#[repr(transparent)]
#[derive(RefCast, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State<T: Send + Sync + 'static>(T);

impl<T: Send + Sync + 'static> State<T> {
    /// Returns the managed state value in `rocket` for the type `T` if it is
    /// being managed by `rocket`. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::State;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Managed(usize);
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Unmanaged(usize);
    ///
    /// let rocket = rocket::build().manage(Managed(7));
    ///
    /// let state: Option<&State<Managed>> = State::get(&rocket);
    /// assert_eq!(state.map(|s| s.inner()), Some(&Managed(7)));
    ///
    /// let state: Option<&State<Unmanaged>> = State::get(&rocket);
    /// assert_eq!(state, None);
    /// ```
    #[inline(always)]
    pub fn get<P: Phase>(rocket: &Rocket<P>) -> Option<&State<T>> {
        rocket.state::<T>().map(State::ref_cast)
    }

    /// This exists because `State::from()` would otherwise be nothing. But we
    /// want `State::from(&foo)` to give us `<&State>::from(&foo)`. Here it is.
    #[doc(hidden)]
    #[inline(always)]
    pub fn from(value: &T) -> &State<T> {
        State::ref_cast(value)
    }

    /// Borrow the inner value.
    ///
    /// Using this method is typically unnecessary as `State` implements
    /// [`Deref`] with a [`Deref::Target`] of `T`. This means Rocket will
    /// automatically coerce a `State<T>` to an `&T` as required. This method
    /// should only be used when a longer lifetime is required.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::State;
    ///
    /// #[derive(Clone)]
    /// struct MyConfig {
    ///     user_val: String
    /// }
    ///
    /// fn handler1<'r>(config: &State<MyConfig>) -> String {
    ///     let config = config.inner().clone();
    ///     config.user_val
    /// }
    ///
    /// // Use the `Deref` implementation which coerces implicitly
    /// fn handler2(config: &State<MyConfig>) -> String {
    ///     config.user_val.clone()
    /// }
    /// ```
    #[inline(always)]
    pub fn inner(&self) -> &T {
        &self.0
    }
}

impl<'r, T: Send + Sync + 'static> From<&'r T> for &'r State<T> {
    #[inline(always)]
    fn from(reference: &'r T) -> Self {
        State::ref_cast(reference)
    }
}

#[crate::async_trait]
impl<'r, T: Send + Sync + 'static> FromRequest<'r> for &'r State<T> {
    type Error = ();

    #[inline(always)]
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, ()> {
        match State::get(req.rocket()) {
            Some(state) => Outcome::Success(state),
            None => {
                error_!("Attempted to retrieve unmanaged state `{}`!", type_name::<T>());
                Outcome::Error((Status::InternalServerError, ()))
            }
        }
    }
}

impl<T: Send + Sync + 'static> Sentinel for &State<T> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        if rocket.state::<T>().is_none() {
            let type_name = type_name::<T>();
            error!("launching with unmanaged `{}` state.", type_name.primary().bold());
            info_!("Using `State` requires managing it with `.manage()`.");
            return true;
        }

        false
    }
}

impl<T: Send + Sync + fmt::Display + 'static> fmt::Display for State<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}
