use std::io;
use std::os::fd::AsRawFd;

pub fn lock_exclusive_nonblocking<T: AsRawFd>(file: &T) -> io::Result<()> {
    let raw_fd = file.as_raw_fd();
    let res = unsafe { libc::flock(raw_fd, libc::LOCK_EX | libc::LOCK_NB) };

    match res {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn unlock_nonblocking<T: AsRawFd>(file: &T) -> io::Result<()> {
    let res = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN | libc::LOCK_NB) };

    match res {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}
