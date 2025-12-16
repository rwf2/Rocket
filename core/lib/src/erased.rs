use std::io;
use std::mem::transmute;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use http::request::Parts;
use tokio::io::{AsyncRead, ReadBuf};

use crate::data::{Data, IoHandler, RawStream};
use crate::{Orbit, Request, Response, Rocket};

// TODO: Magic with trait async fn to get rid of the box pin.
// TODO: Write safety proofs.

macro_rules! static_assert_covariance {
    ($($T:tt)*) => (
        const _: () = {
            fn _assert_covariance<'x: 'y, 'y>(x: &'y $($T)*<'x>) -> &'y $($T)*<'y> { x }
        };
    )
}

#[derive(Debug)]
pub struct ErasedRequest {
    // XXX: SAFETY: This (dependent) field must come first due to drop order!
    request: Request<'static>,
    _rocket: Arc<Rocket<Orbit>>,
    _parts: Box<Parts>,
}

impl Drop for ErasedRequest {
    fn drop(&mut self) {}
}

#[derive(Debug)]
pub struct ErasedResponse {
    // XXX: SAFETY: This (dependent) field must come first due to drop order!
    response: Response<'static>,
    _request: Arc<ErasedRequest>,
}

impl Drop for ErasedResponse {
    fn drop(&mut self) {}
}

pub struct ErasedIoHandler {
    // XXX: SAFETY: This (dependent) field must come first due to drop order!
    io: Box<dyn IoHandler + 'static>,
    _request: Arc<ErasedRequest>,
}

impl Drop for ErasedIoHandler {
    fn drop(&mut self) {}
}

impl ErasedRequest {
    pub fn new(
        rocket: Arc<Rocket<Orbit>>,
        parts: Parts,
        constructor: impl for<'r> FnOnce(&'r Rocket<Orbit>, &'r Parts) -> Request<'r>,
    ) -> ErasedRequest {
        let rocket: Arc<Rocket<Orbit>> = rocket;
        let parts: Box<Parts> = Box::new(parts);
        let request: Request<'_> = {
            let rocket: &Rocket<Orbit> = &rocket;
            let rocket: &'static Rocket<Orbit> = unsafe { transmute(rocket) };
            let parts: &Parts = &parts;
            let parts: &'static Parts = unsafe { transmute(parts) };
            constructor(rocket, parts)
        };

        ErasedRequest {
            _rocket: rocket,
            _parts: parts,
            request,
        }
    }

    pub fn inner(&self) -> &Request<'_> {
        static_assert_covariance!(Request);
        &self.request
    }

    pub async fn into_response<T, D>(
        self,
        raw_stream: D,
        preprocess: impl for<'r, 'x> FnOnce(
            &'r Rocket<Orbit>,
            &'r mut Request<'x>,
            &'r mut Data<'x>,
        ) -> BoxFuture<'r, T>,
        dispatch: impl for<'r> FnOnce(
            T,
            &'r Rocket<Orbit>,
            &'r Request<'r>,
            Data<'r>,
        ) -> BoxFuture<'r, Response<'r>>,
    ) -> ErasedResponse
    where
        T: Send + Sync + 'static,
        D: for<'r> Into<RawStream<'r>>,
    {
        let mut data: Data<'_> = Data::from(raw_stream);
        let mut parent = Arc::new(self);
        let token: T = {
            let parent: &mut ErasedRequest = Arc::get_mut(&mut parent).unwrap();
            let rocket: &Rocket<Orbit> = &parent._rocket;
            let request: &mut Request<'_> = &mut parent.request;
            let data: &mut Data<'_> = &mut data;
            preprocess(rocket, request, data).await
        };

        let parent = parent;
        let response: Response<'_> = {
            let parent: &ErasedRequest = &parent;
            let parent: &'static ErasedRequest = unsafe { transmute(parent) };
            let rocket: &Rocket<Orbit> = &parent._rocket;
            let request: &Request<'_> = &parent.request;
            dispatch(token, rocket, request, data).await
        };

        ErasedResponse {
            _request: parent,
            response,
        }
    }
}

impl ErasedResponse {
    pub fn inner(&self) -> &Response<'_> {
        static_assert_covariance!(Response);
        &self.response
    }

    pub fn with_inner_mut<'a, T>(
        &'a mut self,
        f: impl for<'r> FnOnce(&'a mut Response<'r>) -> T,
    ) -> T {
        static_assert_covariance!(Response);
        f(&mut self.response)
    }

    pub fn make_io_handler<'a, T: 'static>(
        &'a mut self,
        constructor: impl for<'r> FnOnce(
            &'r Request<'r>,
            &'a mut Response<'r>,
        ) -> Option<(T, Box<dyn IoHandler + 'r>)>,
    ) -> Option<(T, ErasedIoHandler)> {
        let parent: Arc<ErasedRequest> = self._request.clone();
        let io: Option<(T, Box<dyn IoHandler + '_>)> = {
            let parent: &ErasedRequest = &parent;
            let parent: &'static ErasedRequest = unsafe { transmute(parent) };
            let request: &Request<'_> = &parent.request;
            constructor(request, &mut self.response)
        };

        io.map(|(v, io)| {
            (
                v,
                ErasedIoHandler {
                    _request: parent,
                    io,
                },
            )
        })
    }
}

impl ErasedIoHandler {
    pub fn with_inner_mut<'a, T: 'a>(
        &'a mut self,
        f: impl for<'r> FnOnce(&'a mut Box<dyn IoHandler + 'r>) -> T,
    ) -> T {
        fn _assert_covariance<'x: 'y, 'y>(x: &'y (dyn IoHandler + 'x)) -> &'y (dyn IoHandler + 'y) {
            x
        }

        f(&mut self.io)
    }

    pub fn take<'a>(&'a mut self) -> Box<dyn IoHandler + 'a> {
        fn _assert_covariance<'x: 'y, 'y>(x: &'y (dyn IoHandler + 'x)) -> &'y (dyn IoHandler + 'y) {
            x
        }

        self.with_inner_mut(|handler| std::mem::replace(handler, Box::new(())))
    }
}

impl AsyncRead for ErasedResponse {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.get_mut()
            .with_inner_mut(|r| Pin::new(r.body_mut()).poll_read(cx, buf))
    }
}
