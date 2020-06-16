use std::ops::{Deref, DerefMut};
use std::io;
use std::iter::FromIterator;

use tokio_io::AsyncReadExt;

use rocket::request::Request;
use rocket::outcome::Outcome::*;
use rocket::data::{Transform::*, Transformed, Data, FromData, TransformFuture, FromDataFuture};
use rocket::response::{self, Responder, content};
use rocket::http::Status;

use serde::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};


#[derive(Debug)]
pub struct Ron<T>(pub T);

impl<T> Ron<T> {
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}


/// Default limit for RON is 1MB.
const LIMIT: u64 = 1 << 20;

#[derive(Debug)]
pub enum RonError<'a> {
    Io(io::Error),

    Parse(&'a str, ron::de::Error),
}

impl<'a, T: Deserialize<'a>> FromData<'a> for Ron<T> {
    type Error = RonError<'a>;
    type Owned = String;
    type Borrowed= str;
 
    fn transform(r: &Request<'_>, d: Data) -> TransformFuture<'a, Self::Owned, Self::Error> {
        let size_limit = r.limits().get("ron").unwrap_or(LIMIT);
        Box::pin(async move {
            let mut s = String::with_capacity(512);
            let mut reader = d.open().take(size_limit);
            match reader.read_to_string(&mut s).await {
                Ok(_) => Borrowed(Success(s)),
                Err(e) => Borrowed(Failure((Status::BadRequest, RonError::Io(e))))
            }
        })
    }

    fn from_data(_: &Request<'_>, o: Transformed<'a, Self>) -> FromDataFuture<'a, Self, Self::Error> {
        Box::pin(async move {
            let string = try_outcome!(o.borrowed());
            match ron::de::from_str(&string) {
                Ok(v) => Success(Ron(v)),
                Err(e) => {
                    error_!("Couldn't parse RON body: {:?}", e);
                    Failure((Status::BadRequest, RonError::Parse(string, e)))
                }
            }
        })
    }
}


impl<'r, T: Serialize> Responder<'r> for Ron<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::ResultFuture<'r> {
        match ron::ser::to_string(&self.0) {
            Ok(string) => Box::pin(async move { Ok(content::Ron(string).respond_to(req).await.unwrap()) }),
            Err(e) => Box::pin (async move {
                error_!("JSON failed to serialize: {:?}", e);
                Err(Status::InternalServerError)
            })
        }
    }
}


impl<T> Deref for Ron<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Ron<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}


