use crate::error::{LaunchError, LaunchErrorKind};
use std::path::Path;
use std::fs::{self, File};

crate fn lock_socket<P: AsRef<Path>>(path: P) -> Result<File, LaunchError> {
    use fs2::FileExt;

    let path = path.as_ref();
    let lock_path = path.with_extension("lock");

    let lock_file = fs::OpenOptions::new()
        .read(true).write(true).create(true)
        .open(&lock_path)?;

    lock_file
        .try_lock_exclusive()
        .map_err(|e| LaunchError::new(LaunchErrorKind::FailedSocketLock(lock_path, e)))?;

    if path.exists() {
        fs::remove_file(path)?;
    }

    // On Unix, the file remains until the last handle to it is closed. We
    // return the handle to the user. When it is dropped, the lock will free.
    Ok(lock_file)
}

#[cfg(not(feature = "tls"))]
macro_rules! serve_unix_socket {
    ($rocket:expr, |$server:ident, $proto:ident| $continue:expr) => ({
        use $crate::http::unix::UnixSocketServer;
        use $crate::config::Address;

        let path = $rocket.config.full_address();
        let _lock_file = match $crate::unix::lock_socket(&path) {
            Ok(file) => file,
            Err(e) => return e
        };

        let ($proto, $server) = (Address::UNIX_PREFIX, UnixSocketServer::http(path));
        $continue
    })
}

#[cfg(feature = "tls")]
macro_rules! serve_unix_socket {
    ($rocket:expr, |$server:ident, $proto:ident| $continue:expr) => ({
        use $crate::http::unix::UnixSocketServer;
        use $crate::config::Address;

        let path = $rocket.config.full_address();
        let _lock_file = match $crate::unix::lock_socket(&path) {
            Ok(file) => file,
            Err(e) => return e
        };

        if let Some(tls) = $rocket.config.tls.clone() {
            let tls = $crate::http::tls::TlsServer::new(tls.certs, tls.key);
            let ($proto, $server) = (Address::UNIX_PREFIX, UnixSocketServer::https(path, tls));
            $continue
        } else {
            let ($proto, $server) = (Address::UNIX_PREFIX, UnixSocketServer::http(path));
            $continue
        }
    })
}
