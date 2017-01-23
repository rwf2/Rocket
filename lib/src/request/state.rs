use std::ops::Deref;

use request::{self, FromRequest, Request};
use outcome::Outcome;
use http::Status;

// TODO: Doc.
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
