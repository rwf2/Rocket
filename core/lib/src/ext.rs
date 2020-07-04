use std::io;

fn read_max_internal<T: io::Read>(reader: &mut T, mut buf: &mut [u8])
                                  -> io::Result<usize> {
    let start_len = buf.len();
    while !buf.is_empty() {
        match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    };

    Ok(start_len - buf.len())
}

pub trait ReadExt: io::Read + Sized {
    fn read_max(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(read_max_internal(self, buf)?)
    }
}

impl<T: io::Read> ReadExt for T {  }
