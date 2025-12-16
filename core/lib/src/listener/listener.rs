#![allow(unexpected_cfgs)]

use std::io;

use futures::TryFutureExt;
use tokio_util::either::Either;

use crate::listener::{Connection, Endpoint};

pub trait Listener: Sized + Send + Sync {
    type Accept: Send;

    type Connection: Connection;

    #[crate::async_bound(Send)]
    async fn accept(&self) -> io::Result<Self::Accept>;

    #[crate::async_bound(Send)]
    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection>;

    fn endpoint(&self) -> io::Result<Endpoint>;
}

impl<L: Listener> Listener for &L {
    type Accept = L::Accept;

    type Connection = L::Connection;

    async fn accept(&self) -> io::Result<Self::Accept> {
        <L as Listener>::accept(self).await
    }

    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection> {
        <L as Listener>::connect(self, accept).await
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        <L as Listener>::endpoint(self)
    }
}

impl<A: Listener, B: Listener> Listener for Either<A, B> {
    type Accept = Either<A::Accept, B::Accept>;

    type Connection = Either<A::Connection, B::Connection>;

    async fn accept(&self) -> io::Result<Self::Accept> {
        match self {
            Either::Left(l) => l.accept().map_ok(Either::Left).await,
            Either::Right(l) => l.accept().map_ok(Either::Right).await,
        }
    }

    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection> {
        match (self, accept) {
            (Either::Left(l), Either::Left(a)) => l.connect(a).map_ok(Either::Left).await,
            (Either::Right(l), Either::Right(a)) => l.connect(a).map_ok(Either::Right).await,
            _ => unreachable!(),
        }
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        match self {
            Either::Left(l) => l.endpoint(),
            Either::Right(l) => l.endpoint(),
        }
    }
}
