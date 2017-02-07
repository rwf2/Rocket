use std::io::{self, Read, BufRead, Write, Cursor, BufReader};
use std::path::Path;
use std::fs::File;

use http::hyper::h1::HttpReader;
use http::hyper::net::NetworkStream;
use http::hyper::buffer;

pub type BodyReader<'a, 'b> =
    self::HttpReader<&'a mut self::buffer::BufReader<&'b mut NetworkStream>>;

const PEEK_BYTES: usize = 4096;

pub struct DataStream {
    stream: BufReader<Cursor<Vec<u8>>>,
}

impl Read for DataStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl BufRead for DataStream {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.stream.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.stream.consume(amt)
    }
}

pub struct Data {
    data: Vec<u8>,
}

impl Data {
    pub fn open(self) -> DataStream {
        DataStream { stream: BufReader::new(Cursor::new(self.data)) }
    }

    #[inline]
    pub fn peek(&self) -> &[u8] {
        &self.data[..::std::cmp::min(PEEK_BYTES, self.data.len())]
    }

    #[inline]
    pub fn peek_complete(&self) -> bool {
        self.data.len() <= PEEK_BYTES
    }

    #[inline]
    pub fn stream_to<W: Write>(self, writer: &mut W) -> io::Result<u64> {
        io::copy(&mut self.open(), writer)
    }

    #[inline]
    pub fn stream_to_file<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        io::copy(&mut self.open(), &mut File::create(path)?)
    }

    pub(crate) fn from_hyp(mut h_body: BodyReader) -> Result<Data, &'static str> {
        let mut vec = Vec::new();
        if let Err(_) = io::copy(&mut h_body, &mut vec) {
            return Err("Reading from body failed.");
        };

        Ok(Data::new(vec))
    }

    pub(crate) fn new(data: Vec<u8>) -> Data {
        Data { data: data }
    }
}
