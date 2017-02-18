use std::ops::Deref;

use request::{self, FromRequest, Request};
use outcome::Outcome;
use http::Status;

/// Request guard to retrieve managed state.
///
/// This type can be used as a request guard to retrieve the state Rocket is
/// managing for some type `T`. This allows for the sharing of state across any
/// number of handlers. A value for the given type must previously have been
/// registered to be managed by Rocket via the
/// [manage](/rocket/struct.Rocket.html#method.manage) method. The type being
/// managed must be thread safe and sendable across thread boundaries. In other
/// words, it must implement `Send + Sync + 'static`.
///
/// # Example
///
/// Imagine you have some configuration struct of the type `MyConfig` that you'd
/// like to initialize at start-up and later access it in several handlers. The
/// following example does just this:
///
/// ```rust
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// use rocket::State;
///
/// // In a real application, this would likely be more complex.
/// struct MyConfig(String);
///
/// #[get("/")]
/// fn index(state: State<MyConfig>) -> String {
///     format!("The config value is: {}", state.0)
/// }
///
/// #[get("/raw")]
/// fn raw_config_value<'r>(state: State<'r, MyConfig>) -> &'r str {
///     // use `inner()` to get a lifetime longer than `deref` gives us
///     state.inner().0.as_str()
/// }
///
/// fn main() {
///     let config = MyConfig("user input".to_string());
/// # if false { // We don't actually want to launch the server in an example.
///     rocket::ignite()
///         .mount("/", routes![index, raw_config_value])
///         .manage(config)
///         .launch()
/// # }
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct State<'r, T: Send + Sync + 'static>(&'r T);

impl<'r, T: Send + Sync + 'static> State<'r, T> {
    /// Retrieve a borrow to the underyling value.
    ///
    /// Using this method is typically unnecessary as `State` implements `Deref`
    /// with a `Target` of `T`. This means Rocket will automatically coerce a
    /// `State<T>` to an `&T` when the types call for it.
    pub fn inner(&self) -> &'r T {
        self.0
    }
}

// TODO: Doc.
impl<'a, 'r, T: Send + Sync + 'static> FromRequest<'a, 'r> for State<'r, T> {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<State<'r, T>, ()> {
        if let Some(state) = req.get_state() {
            match state.try_get::<T>() {
                Some(state) => Outcome::Success(State(state)),
                None => {
                    error_!("Attempted to retrieve unmanaged state!");
                    Outcome::Failure((Status::InternalServerError, ()))
                }
            }
        } else {
            error_!("Internal Rocket error: managed state is unset!");
            error_!("Please report this error in the Rocket GitHub issue tracker.");
            Outcome::Failure((Status::InternalServerError, ()))
        }
    }
}

impl<'r, T: Send + Sync + 'static> Deref for State<'r, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}
