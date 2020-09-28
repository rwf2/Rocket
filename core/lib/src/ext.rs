use std::io;

fn read_max_internal<T: io::Read>(reader: &mut T, mut buf: &mut [u8],
                                  wouldblock_flush_signalling: bool)
                                  -> io::Result<(usize, bool)> {
    let start_len = buf.len();
    let need_flush = loop {
        if buf.is_empty() { break false }
        match reader.read(buf) {
            Ok(0) => { break true }
            Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(ref e) if (e.kind() == io::ErrorKind::WouldBlock &&
                           wouldblock_flush_signalling) => { break true }
            Err(e) => return Err(e),
        }
    };

    Ok((start_len - buf.len(), need_flush))
}

pub trait ReadExt: io::Read + Sized {
    fn read_max(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(read_max_internal(self, buf, false)?.0)
    }

    /// Tries to fill buf with data.  Short reads can occur for EOF or
    /// flush requests.  With SSE enabled, a flush request occurs if
    /// the underlying reader returns ErrorKind::Wouldblock
    fn read_max_wfs(&mut self, buf: &mut [u8])
                    -> io::Result<(usize, bool)> {
        read_max_internal(self, buf, cfg!(feature="sse"))
    }
}

impl<T: io::Read> ReadExt for T {  }
