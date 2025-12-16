use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Peekable<const N: usize, R> {
    pub(crate) buffer: Vec<u8>,
    pub(crate) complete: bool,
    pub(crate) reader: R,
}

impl<const N: usize, R: AsyncRead + Unpin> Peekable<N, R> {
    pub fn new(reader: R) -> Self {
        Self {
            buffer: Vec::new(),
            complete: false,
            reader,
        }
    }

    pub fn with_buffer(buffer: Vec<u8>, complete: bool, reader: R) -> Self {
        Self {
            buffer,
            complete,
            reader,
        }
    }

    pub async fn peek(&mut self, num: usize) -> &[u8] {
        if self.complete {
            return self.buffer.as_slice();
        }

        let to_read = std::cmp::min(N, num);
        if self.buffer.len() >= to_read {
            return self.buffer.as_slice();
        }

        if self.buffer.capacity() == 0 {
            self.buffer.reserve(N);
        }

        while self.buffer.len() < to_read {
            match self.reader.read_buf::<Vec<u8>>(&mut self.buffer).await {
                Ok(0) => {
                    self.complete = self.buffer.capacity() > self.buffer.len();
                    break;
                }
                Ok(_) => { /* continue */ }
                Err(e) => {
                    error!("failed to read into peek buffer: {:?}.", e);
                    break;
                }
            }
        }

        self.buffer.as_slice()
    }
}
