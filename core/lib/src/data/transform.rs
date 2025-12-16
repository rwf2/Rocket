use std::io;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::ReadBuf;

/// Chainable, in-place, streaming data transformer.
///
/// [`Transform`] operates on [`TransformBuf`]s similar to how [`AsyncRead`]
/// operats on [`ReadBuf`]. A [`Transform`] sits somewhere in a chain of
/// transforming readers. The head (most upstream part) of the chain is _always_
/// an [`AsyncRead`]: the data source. The tail (all downstream parts) is
/// composed _only_ of other [`Transform`]s:
///
/// ```text
///                          downstream --->
///  AsyncRead | Transform | .. | Transform
/// <---- upstream
/// ```
///
/// When the upstream source makes data available, the
/// [`Transform::transform()`] method is called. [`Transform`]s may obtain the
/// subset of the filled section added by an upstream data source with
/// [`TransformBuf::fresh()`]. They may modify this data at will, potentially
/// changing the size of the filled section. For example,
/// [`TransformBuf::spoil()`] "removes" all of the fresh data, and
/// [`TransformBuf::fresh_mut()`] can be used to modify the data in-place.
///
/// Additionally, new data may be added in-place via the traditional approach:
/// write to (or overwrite) the initialized section of the buffer and mark it as
/// filled. All of the remaining filled data will be passed to downstream
/// transforms as "fresh" data. To add data to the end of the (potentially
/// rewritten) stream, the [`Transform::poll_finish()`] method can be
/// implemented.
///
/// [`AsyncRead`]: tokio::io::AsyncRead
pub trait Transform {
    /// Called when data is read from the upstream source. For any given fresh
    /// data, this method is called only once. [`TransformBuf::fresh()`] is
    /// guaranteed to contain at least one byte.
    ///
    /// While this method is not _async_ (it does not return [`Poll`]), it is
    /// nevertheless executed in an async context and should respect all such
    /// restrictions including not blocking.
    fn transform(self: Pin<&mut Self>, buf: &mut TransformBuf<'_, '_>) -> io::Result<()>;

    /// Called when the upstream is finished, that is, it has no more data to
    /// fill. At this point, the transform becomes an async reader. This method
    /// thus has identical semantics to [`AsyncRead::poll_read()`]. This method
    /// may never be called if the upstream does not finish.
    ///
    /// The default implementation returns `Poll::Ready(Ok(()))`.
    ///
    /// [`AsyncRead::poll_read()`]: tokio::io::AsyncRead::poll_read()
    fn poll_finish(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let (_, _) = (cx, buf);
        Poll::Ready(Ok(()))
    }
}

/// A buffer of transformable streaming data.
///
/// # Overview
///
/// A byte buffer, similar to a [`ReadBuf`], with a "fresh" dimension. Fresh
/// data is always a subset of the filled data, filled data is always a subset
/// of initialized data, and initialized data is always a subset of the buffer
/// itself. Both the filled and initialized data sections are guaranteed to be
/// at the start of the buffer, but the fresh subset is likely to begin
/// somewhere inside the filled section.
///
/// To visualize this, the diagram below represents a possible state for the
/// byte buffer being tracked. The square `[ ]` brackets represent the complete
/// buffer, while the curly `{ }` represent the named subset.
///
/// ```text
/// [  { !! fresh !! }                                 ]
/// { +++ filled +++ }          unfilled               ]
/// { ----- initialized ------ }     uninitialized     ]
/// [                    capacity                      ]
/// ```
///
/// The same buffer represented in its true single dimension is below:
///
/// ```text
/// [ ++!!!!!!!!!!!!!!---------xxxxxxxxxxxxxxxxxxxxxxxx]
/// ```
///
/// * `+`: filled (implies initialized)
/// * `!`: fresh (implies filled)
/// * `-`: unfilled / initialized (implies initialized)
/// * `x`: uninitialized (implies unfilled)
///
/// As with [`ReadBuf`], [`AsyncRead`] readers fill the initialized portion of a
/// [`TransformBuf`] to indicate that data is available. _Filling_ initialized
/// portions of the byte buffers is what increases the size of the _filled_
/// section. Because a [`ReadBuf`] may already be partially filled when a reader
/// adds bytes to it, a mechanism to track where the _newly_ filled portion
/// exists is needed. This is exactly what the "fresh" section tracks.
///
/// [`AsyncRead`]: tokio::io::AsyncRead
pub struct TransformBuf<'a, 'b> {
    pub(crate) buf: &'a mut ReadBuf<'b>,
    pub(crate) cursor: usize,
}

impl TransformBuf<'_, '_> {
    /// Returns a borrow to the fresh data: data filled by the upstream source.
    pub fn fresh(&self) -> &[u8] {
        &self.filled()[self.cursor..]
    }

    /// Returns a mutable borrow to the fresh data: data filled by the upstream
    /// source.
    pub fn fresh_mut(&mut self) -> &mut [u8] {
        let cursor = self.cursor;
        &mut self.filled_mut()[cursor..]
    }

    /// Spoils the fresh data by resetting the filled section to its value
    /// before any new data was added. As a result, the data will never be seen
    /// by any downstream consumer unless it is returned via another mechanism.
    pub fn spoil(&mut self) {
        let cursor = self.cursor;
        self.set_filled(cursor);
    }
}

pub struct Inspect(pub(crate) Box<dyn FnMut(&[u8]) + Send + Sync + 'static>);

impl Transform for Inspect {
    fn transform(mut self: Pin<&mut Self>, buf: &mut TransformBuf<'_, '_>) -> io::Result<()> {
        (self.0)(buf.fresh());
        Ok(())
    }
}

pub struct InPlaceMap(
    pub(crate) Box<dyn FnMut(&mut TransformBuf<'_, '_>) -> io::Result<()> + Send + Sync + 'static>,
);

impl Transform for InPlaceMap {
    fn transform(mut self: Pin<&mut Self>, buf: &mut TransformBuf<'_, '_>) -> io::Result<()> {
        (self.0)(buf)
    }
}

impl<'a, 'b> Deref for TransformBuf<'a, 'b> {
    type Target = ReadBuf<'b>;

    fn deref(&self) -> &Self::Target {
        self.buf
    }
}

impl<'a, 'b> DerefMut for TransformBuf<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buf
    }
}

// TODO: Test chaining various transform combinations:
//  * consume | consume
//  * add | consume
//  * consume | add
//  * add | add
// Where `add` is a transformer that adds data to the stream, and `consume` is
// one that removes data.
#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::hash::SipHasher;
    use std::sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc,
    };

    use parking_lot::Mutex;
    use ubyte::ToByteUnit;

    use crate::fairing::AdHoc;
    use crate::http::Method;
    use crate::local::blocking::Client;
    use crate::{route, Data, Request, Response, Route};

    mod hash_transform {
        use std::hash::Hasher;
        use std::io::Cursor;

        use tokio::io::AsyncRead;

        use super::super::*;

        pub struct HashTransform<H: Hasher> {
            pub(crate) hasher: H,
            pub(crate) hash: Option<Cursor<[u8; 8]>>,
        }

        impl<H: Hasher + Unpin> Transform for HashTransform<H> {
            fn transform(
                mut self: Pin<&mut Self>,
                buf: &mut TransformBuf<'_, '_>,
            ) -> io::Result<()> {
                self.hasher.write(buf.fresh());
                buf.spoil();
                Ok(())
            }

            fn poll_finish(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                if self.hash.is_none() {
                    let hash = self.hasher.finish();
                    self.hash = Some(Cursor::new(hash.to_be_bytes()));
                }

                let cursor = self.hash.as_mut().unwrap();
                Pin::new(cursor).poll_read(cx, buf)
            }
        }

        impl crate::Data<'_> {
            /// Chain an in-place hash [`Transform`] to `self`.
            pub fn chain_hash_transform<H: std::hash::Hasher>(&mut self, hasher: H) -> &mut Self
            where
                H: Unpin + Send + Sync + 'static,
            {
                self.chain_transform(HashTransform { hasher, hash: None })
            }
        }
    }

    #[test]
    fn test_transform_series() {
        fn handler<'r>(_: &'r Request<'_>, data: Data<'r>) -> route::BoxFuture<'r> {
            Box::pin(async move {
                data.open(128.bytes())
                    .stream_to(tokio::io::sink())
                    .await
                    .expect("read ok");
                route::Outcome::Success(Response::new())
            })
        }

        let inspect2: Arc<AtomicU8> = Arc::new(AtomicU8::new(0));
        let raw_data: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let hash: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let rocket = crate::build()
            .manage(hash.clone())
            .manage(raw_data.clone())
            .manage(inspect2.clone())
            .mount("/", vec![Route::new(Method::Post, "/", handler)])
            .attach(AdHoc::on_request("transforms", |req, data| {
                Box::pin(async {
                    let hash1 = req.rocket().state::<Arc<AtomicU64>>().cloned().unwrap();
                    let hash2 = req.rocket().state::<Arc<AtomicU64>>().cloned().unwrap();
                    let raw_data = req
                        .rocket()
                        .state::<Arc<Mutex<Vec<u8>>>>()
                        .cloned()
                        .unwrap();
                    let inspect2 = req.rocket().state::<Arc<AtomicU8>>().cloned().unwrap();
                    data.chain_inspect(move |bytes| {
                        *raw_data.lock() = bytes.to_vec();
                    })
                    .chain_hash_transform(SipHasher::new())
                    .chain_inspect(move |bytes| {
                        assert_eq!(bytes.len(), 8);
                        let bytes: [u8; 8] = bytes.try_into().expect("[u8; 8]");
                        let value = u64::from_be_bytes(bytes);
                        hash1.store(value, Ordering::Release);
                    })
                    .chain_inspect(move |bytes| {
                        assert_eq!(bytes.len(), 8);
                        let bytes: [u8; 8] = bytes.try_into().expect("[u8; 8]");
                        let value = u64::from_be_bytes(bytes);
                        let prev = hash2.load(Ordering::Acquire);
                        assert_eq!(prev, value);
                        inspect2.fetch_add(1, Ordering::Release);
                    });
                })
            }));

        // Make sure nothing has happened yet.
        assert!(raw_data.lock().is_empty());
        assert_eq!(hash.load(Ordering::Acquire), 0);
        assert_eq!(inspect2.load(Ordering::Acquire), 0);

        // Check that nothing happens if the data isn't read.
        let client = Client::debug(rocket).unwrap();
        client.get("/").body("Hello, world!").dispatch();
        assert!(raw_data.lock().is_empty());
        assert_eq!(hash.load(Ordering::Acquire), 0);
        assert_eq!(inspect2.load(Ordering::Acquire), 0);

        // Check inspect + hash + inspect + inspect.
        client.post("/").body("Hello, world!").dispatch();
        assert_eq!(raw_data.lock().as_slice(), "Hello, world!".as_bytes());
        assert_eq!(hash.load(Ordering::Acquire), 0xae5020d7cf49d14f);
        assert_eq!(inspect2.load(Ordering::Acquire), 1);

        // Check inspect + hash + inspect + inspect, round 2.
        let string = "Rocket, Rocket, where art thee? Oh, tis in the sky, I see!";
        client.post("/").body(string).dispatch();
        assert_eq!(raw_data.lock().as_slice(), string.as_bytes());
        assert_eq!(hash.load(Ordering::Acquire), 0x323f9aa98f907faf);
        assert_eq!(inspect2.load(Ordering::Acquire), 2);
    }
}
