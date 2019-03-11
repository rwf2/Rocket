use std::io;
use std::pin::Pin;

use futures::io::{AsyncRead, AsyncReadExt as _};
use futures::future::{Future};
use futures::task::{Poll, Context};

// Based on std::io::Take, but for AsyncRead instead of Read
pub struct Take<R>{
    inner: R,
    limit: u64,
}

// TODO.async: Verify correctness of this implementation.
impl<R> AsyncRead for Take<R> where R: AsyncRead + Unpin {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, io::Error>> {
        if self.limit == 0 {
            return Poll::Ready(Ok(0));
        }

        let max = std::cmp::min(buf.len() as u64, self.limit) as usize;
        match Pin::new(&mut self.inner).poll_read(cx, &mut buf[..max]) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(n)) => {
                self.limit -= n as u64;
                Poll::Ready(Ok(n))
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }
}

pub trait AsyncReadExt: AsyncRead {
    fn take(self, limit: u64) -> Take<Self> where Self: Sized {
        Take { inner: self, limit }
    }

    // TODO.async: Verify correctness of this implementation.
    fn read_max<'a>(&'a mut self, mut buf: &'a mut [u8]) -> Pin<Box<dyn Future<Output=io::Result<usize>> + Send + '_>>
        where Self: Send + Unpin
    {
        Box::pin(async move {
            let start_len = buf.len();
            while !buf.is_empty() {
                match self.read(buf).await {
                    Ok(0) => break,
                    Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e),
                }
            }

            Ok(start_len - buf.len())
        })
    }
}

impl<T: AsyncRead> AsyncReadExt for T { }
