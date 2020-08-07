use std::ops::Deref;

use crate::rocket::Cargo;
use crate::request::{self, FromRequest, Request};
use crate::outcome::Outcome;
use crate::http::Status;

/// Request guard to retrieve managed state.
///
/// This type can be used as a request guard to retrieve the state Rocket is
/// managing for some type `T`. This allows for the sharing of state across any
/// number of handlers. A value for the given type must previously have been
/// registered to be managed by Rocket via [`Rocket::manage()`]. The type being
/// managed must be thread safe and sendable across thread boundaries. In other
/// words, it must implement [`Send`] + [`Sync`] + `'static`.
///
/// [`Rocket::manage()`]: crate::Rocket::manage()
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
/// fn index(state: State<MyConfig>) -> String {
///     format!("The config value is: {}", state.user_val)
/// }
///
/// #[get("/raw")]
/// fn raw_config_value<'r>(state: State<'r, MyConfig>) -> &'r str {
///     // use `inner()` to get a lifetime longer than `deref` gives us
///     state.inner().user_val.as_str()
/// }
///
/// #[launch]
/// fn rocket() -> rocket::Rocket {
///     rocket::ignite()
///         .mount("/", routes![index, raw_config_value])
///         .manage(MyConfig { user_val: "user input".to_string() })
/// }
/// ```
///
/// # Within Request Guards
///
/// Because `State` is itself a request guard, managed state can be retrieved
/// from another request guard's implementation. In the following code example,
/// `Item` retrieves the `MyConfig` managed state in its [`FromRequest`]
/// implementation using the [`Request::guard()`] method.
///
/// ```rust
/// use rocket::State;
/// use rocket::request::{self, Request, FromRequest};
///
/// # struct MyConfig{ user_val: String };
/// struct Item(String);
///
/// #[rocket::async_trait]
/// impl<'a, 'r> FromRequest<'a, 'r> for Item {
///     type Error = ();
///
///     async fn from_request(request: &'a Request<'r>) -> request::Outcome<Item, ()> {
///         request.guard::<State<MyConfig>>().await
///             .map(|my_config| Item(my_config.user_val.clone()))
///     }
/// }
/// ```
///
/// # Testing with `State`
///
/// When unit testing your application, you may find it necessary to manually
/// construct a type of `State` to pass to your functions. To do so, use the
/// [`State::from()`] static method:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::State;
///
/// struct MyManagedState(usize);
///
/// #[get("/")]
/// fn handler(state: State<MyManagedState>) -> String {
///     state.0.to_string()
/// }
///
/// # rocket::async_test(async {
/// let mut rocket = rocket::ignite().manage(MyManagedState(127));
/// let state = State::from(rocket.inspect().await).expect("managing `MyManagedState`");
/// assert_eq!(handler(state), "127");
/// # });
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State<'r, T: Send + Sync + 'static>(&'r T);

impl<'r, T: Send + Sync + 'static> State<'r, T> {
    /// Retrieve a borrow to the underlying value with a lifetime of `'r`.
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
    /// struct MyConfig {
    ///     user_val: String
    /// }
    ///
    /// // Use `inner()` to get a lifetime of `'r`
    /// fn handler1<'r>(config: State<'r, MyConfig>) -> &'r str {
    ///     &config.inner().user_val
    /// }
    ///
    /// // Use the `Deref` implementation which coerces implicitly
    /// fn handler2(config: State<MyConfig>) -> String {
    ///     config.user_val.clone()
    /// }
    /// ```
    #[inline(always)]
    pub fn inner(&self) -> &'r T {
        self.0
    }

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
    /// # rocket::async_test(async {
    /// let mut rocket = rocket::ignite().manage(Managed(7));
    /// let cargo = rocket.inspect().await;
    ///
    /// let state: Option<State<Managed>> = State::from(cargo);
    /// assert_eq!(state.map(|s| s.inner()), Some(&Managed(7)));
    ///
    /// let state: Option<State<Unmanaged>> = State::from(cargo);
    /// assert_eq!(state, None);
    /// # });
    /// ```
    #[inline(always)]
    pub fn from(rocket: &'r Cargo) -> Option<Self> {
        rocket.state().map(State)
    }
}

#[crate::async_trait]
impl<'a, 'r, T: Send + Sync + 'static> FromRequest<'a, 'r> for State<'r, T> {
    type Error = ();

    #[inline(always)]
    async fn from_request(req: &'a Request<'r>) -> request::Outcome<State<'r, T>, ()> {
        match req.state.managed.try_get::<T>() {
            Some(state) => Outcome::Success(State(state)),
            None => {
                error_!("Attempted to retrieve unmanaged state!");
                Outcome::Failure((Status::InternalServerError, ()))
            }
        }
    }
}

impl<T: Send + Sync + 'static> Deref for State<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}
