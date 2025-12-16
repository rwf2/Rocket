use std::{io, time::Duration};

use crate::listener::{Endpoint, Listener};

static DURATION: Duration = Duration::from_millis(250);

pub struct Bounced<L> {
    listener: L,
}

pub trait BouncedExt: Sized {
    fn bounced(self) -> Bounced<Self> {
        Bounced { listener: self }
    }
}

impl<L> BouncedExt for L {}

fn is_recoverable(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        |io::ErrorKind::ConnectionRefused| io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

impl<L: Listener + Sync> Bounced<L> {
    #[inline]
    pub async fn accept_next(&self) -> <Self as Listener>::Accept {
        loop {
            match self.listener.accept().await {
                Ok(accept) => return accept,
                Err(e) if is_recoverable(&e) => warn!("recoverable connection error: {e}"),
                Err(e) => {
                    warn!("accept error: {e} [retrying in {}ms]", DURATION.as_millis());
                    tokio::time::sleep(DURATION).await;
                }
            };
        }
    }
}

impl<L: Listener + Sync> Listener for Bounced<L> {
    type Accept = L::Accept;

    type Connection = L::Connection;

    async fn accept(&self) -> io::Result<Self::Accept> {
        Ok(self.accept_next().await)
    }

    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection> {
        self.listener.connect(accept).await
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        self.listener.endpoint()
    }
}
