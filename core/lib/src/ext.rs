use std::io::{self, Cursor};
use std::pin::Pin;
use std::task::{Poll, Context};

use futures::{ready, stream::Stream};
use tokio::io::AsyncRead;

use crate::http::hyper::{self, Bytes, HttpBody};

pub struct IntoBytesStream<R> {
    inner: R,
    buf_size: usize,
    buffer: Vec<u8>,
}

impl<R> Stream for IntoBytesStream<R>
    where R: AsyncRead + Unpin
{
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>{
        debug_assert!(self.buffer.len() == self.buf_size);

        let Self { ref mut inner, ref mut buffer, buf_size } = *self;

        match Pin::new(inner).poll_read(cx, &mut buffer[..]) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Ready(Ok(n)) if n == 0 => Poll::Ready(None),
            Poll::Ready(Ok(n)) => {
                // FIXME(perf).
                let mut next = std::mem::replace(buffer, vec![0; buf_size]);
                next.truncate(n);
                Poll::Ready(Some(Ok(Bytes::from(next))))
            }
        }
    }
}

pub trait AsyncReadExt: AsyncRead + Sized {
    fn into_bytes_stream(self, buf_size: usize) -> IntoBytesStream<Self> {
        IntoBytesStream { inner: self, buf_size, buffer: vec![0; buf_size] }
    }
}

impl<T: AsyncRead> AsyncReadExt for T { }

pub struct AsyncReadBody {
    inner: hyper::Body,
    state: State,
}

enum State {
    Pending,
    Partial(Cursor<Bytes>),
    Done,
}

impl AsyncReadBody {
    pub fn empty() -> Self {
        Self { inner: hyper::Body::empty(), state: State::Done }
    }
}

impl From<hyper::Body> for AsyncReadBody {
    fn from(body: hyper::Body) -> Self {
        Self { inner: body, state: State::Pending }
    }
}

impl AsyncRead for AsyncReadBody {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8]
    ) -> Poll<io::Result<usize>> {
        loop {
            match self.state {
                State::Pending => {
                    match ready!(Pin::new(&mut self.inner).poll_data(cx)) {
                        Some(Ok(bytes)) => {
                            self.state = State::Partial(Cursor::new(bytes));
                        }
                        Some(Err(e)) => {
                            let error = io::Error::new(io::ErrorKind::Other, e);
                            return Poll::Ready(Err(error));
                        }
                        None => self.state = State::Done,
                    }
                },
                State::Partial(ref mut cursor) => {
                    match ready!(Pin::new(cursor).poll_read(cx, buf)) {
                        Ok(n) if n == 0 => self.state = State::Pending,
                        result => return Poll::Ready(result),
                    }
                }
                State::Done => return Poll::Ready(Ok(0)),
            }
        }
    }
}
