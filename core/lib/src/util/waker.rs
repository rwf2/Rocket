use std::{io, thread};
use std::sync::{mpsc, Arc};
use std::task::Poll;
use std::time::Duration;

use futures::{future::poll_fn, task::AtomicWaker};
use tokio::runtime::{Handle, RuntimeFlavor};

/// Keeps an idle worker checking I/O and timers when another worker blocks.
pub struct RuntimeWaker {
    stop_tx: mpsc::Sender<()>,
    thread: Option<thread::JoinHandle<()>>,
}

impl RuntimeWaker {
    pub(crate) fn start(interval: Duration) -> io::Result<Option<Self>> {
        let runtime = Handle::current();
        if interval.is_zero() || runtime.runtime_flavor() != RuntimeFlavor::MultiThread {
            return Ok(None);
        }

        let (stop_tx, stop_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("rocket-runtime-waker".into())
            .spawn(move || {
                let waker = Arc::new(AtomicWaker::new());
                let task = runtime.spawn({
                    let waker = waker.clone();
                    poll_fn(move |cx| {
                        waker.register(cx.waker());
                        Poll::<()>::Pending
                    })
                });

                // Wake from outside Tokio so an idle worker can take over.
                // Reusing one task coalesces ticks when no worker can make
                // progress. See https://github.com/tokio-rs/tokio/issues/4730.
                while let Err(mpsc::RecvTimeoutError::Timeout) = stop_rx.recv_timeout(interval) {
                    waker.wake();
                }

                task.abort();
            })?;

        Ok(Some(Self { stop_tx, thread: Some(thread) }))
    }
}

impl Drop for RuntimeWaker {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn skips_current_thread_runtime() {
        assert!(RuntimeWaker::start(Duration::from_secs(1)).unwrap().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn zero_interval_disables_waker() {
        assert!(RuntimeWaker::start(Duration::ZERO).unwrap().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn shutdown_completes_with_blocked_worker() {
        // Ensure that dropping the waker succeeds quickly even when Tokio's
        // only worker is blocked and the wake interval is long.
        let waker = RuntimeWaker::start(Duration::from_secs(60)).unwrap().unwrap();

        // This test runs outside Tokio's worker pool. The task sends on
        // `start_tx`, then occupies the only worker until we drop `release_tx`.
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (start_tx, start_rx) = mpsc::channel();
        let blocker = tokio::spawn(async move {
            start_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });

        // Wait for the task to start.
        start_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        // Drop the waker on another thread so this test can enforce the timeout.
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            drop(waker);
            let _ = stopped_tx.send(());
        });

        // Receiving on `stopped_rx` proves the waker's destructor returned
        // within the deadline, without Tokio's blocked worker making progress.
        let result = stopped_rx.recv_timeout(Duration::from_secs(2));

        // Drop `release_tx` to unblock the worker. Finish cleanup before
        // asserting, even if the wait timed out.
        drop(release_tx);
        blocker.await.unwrap();
        thread.join().unwrap();
        assert!(result.is_ok());
    }
}
