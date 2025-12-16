use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pin_project! {
    /// Stream for the [`chain`](super::AsyncReadExt::chain) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Chain<T, U> {
        #[pin]
        first: Option<T>,
        #[pin]
        second: U,
    }
}

impl<T, U> Chain<T, U> {
    pub(crate) fn new(first: T, second: U) -> Self {
        Self {
            first: Some(first),
            second,
        }
    }
}

impl<T: AsyncRead, U: AsyncRead> Chain<T, U> {
    /// Gets references to the underlying readers in this `Chain`.
    pub fn get_ref(&self) -> (Option<&T>, &U) {
        (self.first.as_ref(), &self.second)
    }
}

impl<T: AsyncRead, U: AsyncRead> AsyncRead for Chain<T, U> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.as_mut().project();
        if let Some(first) = me.first.as_pin_mut() {
            let init_rem = buf.remaining();
            futures::ready!(first.poll_read(cx, buf))?;
            if buf.remaining() == init_rem {
                self.as_mut().project().first.set(None);
            } else {
                return Poll::Ready(Ok(()));
            }
        }

        let me = self.as_mut().project();
        me.second.poll_read(cx, buf)
    }
}
