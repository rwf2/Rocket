use std::io;
use std::path::{Path, PathBuf};

use either::{Either, Left, Right};
use tokio::time::{sleep, Duration};

use crate::fs::NamedFile;
use crate::listener::{Listener, Bind, Connection, Endpoint};
use crate::util::unix;
use crate::{Ignite, Rocket};

pub use tokio::net::UnixStream;

/// Unix domain sockets listener.
///
/// # Configuration
///
/// Reads the following configuration parameters:
///
/// | parameter | type         | default | note                                      |
/// |-----------|--------------|---------|-------------------------------------------|
/// | `address` | [`Endpoint`] |         | required: must be `unix:path`             |
/// | `reuse`   | boolean      | `true`  | whether to create/reuse/delete the socket |
pub struct UnixListener {
    path: PathBuf,
    lock: Option<NamedFile>,
    listener: tokio::net::UnixListener,
}

impl UnixListener {
    pub async fn bind<P: AsRef<Path>>(path: P, reuse: bool) -> io::Result<Self> {
        let path = path.as_ref();
        let lock = if reuse {
            let lock_ext = match path.extension().and_then(|s| s.to_str()) {
                Some(ext) if !ext.is_empty() => format!("{}.lock", ext),
                _ => "lock".to_string()
            };

            let mut opts = tokio::fs::File::options();
            opts.create(true).write(true);
            let lock_path = path.with_extension(lock_ext);
            let lock_file = NamedFile::open_with(lock_path, &opts).await?;

            unix::lock_exclusive_nonblocking(lock_file.file())?;
            if path.exists() {
                tokio::fs::remove_file(&path).await?;
            }

            Some(lock_file)
        } else {
            None
        };

        // Sometimes, we get `AddrInUse`, even though we've tried deleting the
        // socket. If all is well, eventually the socket will _really_ be gone,
        // and this will succeed. So let's try a few times.
        let mut retries = 5;
        let listener = loop {
            match tokio::net::UnixListener::bind(&path) {
                Ok(listener) => break listener,
                Err(e) if path.exists() && lock.is_none() => return Err(e),
                Err(_) if retries > 0 => {
                    retries -= 1;
                    sleep(Duration::from_millis(100)).await;
                },
                Err(e) => return Err(e),
            }
        };

        Ok(UnixListener { lock, listener, path: path.into() })
    }
}

impl Bind for UnixListener {
    type Error = Either<figment::Error, io::Error>;

    async fn bind(rocket: &Rocket<Ignite>) -> Result<Self, Self::Error> {
        let endpoint = Self::bind_endpoint(rocket)?;
        let path = endpoint.unix()
            .ok_or_else(|| Right(io::Error::other("internal error: invalid endpoint")))?;

        let reuse: Option<bool> = rocket.figment().extract_inner("reuse").map_err(Left)?;
        Ok(Self::bind(path, reuse.unwrap_or(true)).await.map_err(Right)?)
    }

    fn bind_endpoint(rocket: &Rocket<Ignite>) -> Result<Endpoint, Self::Error> {
        let as_pathbuf = |e: Option<&Endpoint>| e.and_then(|e| e.unix().map(|p| p.to_path_buf()));
        Endpoint::fetch(rocket.figment(), "unix", "address", as_pathbuf)
            .map(Endpoint::Unix)
            .map_err(Left)
    }
}

impl Listener for UnixListener {
    type Accept = UnixStream;

    type Connection = Self::Accept;

    async fn accept(&self) -> io::Result<Self::Accept> {
        Ok(self.listener.accept().await?.0)
    }

    async fn connect(&self, accept:Self::Accept) -> io::Result<Self::Connection> {
        Ok(accept)
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        self.listener.local_addr()?.try_into()
    }
}

impl Connection for UnixStream {
    fn endpoint(&self) -> io::Result<Endpoint> {
        self.local_addr()?.try_into()
    }
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        if let Some(lock) = &self.lock {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(lock.path());
            let _ = unix::unlock_nonblocking(lock.file());
        } else {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
