use std::io;
use smallvec::{Array, SmallVec};

pub trait ReadExt: io::Read {
    fn read_max(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let start_len = buf.len();
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        Ok(start_len - buf.len())
    }
}

impl<T: io::Read> ReadExt for T {  }

// TODO: It would be nice if we could somehow have one trait that could give us
// either SmallVec or Vec.
pub trait IntoCollection<T> {
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A>;
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, f: F) -> SmallVec<A>;
}

impl<T> IntoCollection<T> for T {
    #[inline]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        let mut vec = SmallVec::new();
        vec.push(self);
        vec
    }

    #[inline(always)]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        f(self).into_collection()
    }
}

impl<T> IntoCollection<T> for Vec<T> {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        SmallVec::from_vec(self)
    }

    #[inline]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        self.into_iter().map(|item| f(item)).collect()
    }
}

impl<'a, T: Clone> IntoCollection<T> for &'a [T] {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        self.iter().cloned().collect()
    }

    #[inline]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        self.iter().cloned().map(|item| f(item)).collect()
    }
}
