use std::fmt;
use std::{ops::Deref, pin::Pin, future::Future};
use std::task::{Context, Poll};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use tokio::sync::Notify;

#[doc(hidden)]
pub struct State {
    tripped: AtomicBool,
    notify: Notify,
}

#[must_use = "`TripWire` does nothing unless polled or `trip()`ed"]
pub struct TripWire {
    state: Arc<State>,
    // `Notified` is `!Unpin`. Even if we could name it, we'd need to pin it.
    event: Option<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>,
}

impl Deref for TripWire {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl Clone for TripWire {
    fn clone(&self) -> Self {
        TripWire {
            state: self.state.clone(),
            event: None
        }
    }
}

impl fmt::Debug for TripWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TripWire")
            .field("tripped", &self.tripped)
            .finish()
    }
}

impl Future for TripWire {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.tripped.load(Ordering::Acquire) {
            self.event = None;
            return Poll::Ready(());
        }

        if self.event.is_none() {
            let state = self.state.clone();
            self.event = Some(Box::pin(async move {
                let notified = state.notify.notified();
                notified.await
            }));
        }

        if let Some(ref mut event) = self.event {
            if event.as_mut().poll(cx).is_ready() {
                // We need to call `trip()` to avoid a race condition where:
                //   1) many trip wires have seen !self.tripped but have not
                //      polled for `self.event` yet, so are not subscribed
                //   2) trip() is called, adding a permit to `event`
                //   3) some trip wires poll `event` for the first time
                //   4) one of those wins, returns `Ready()`
                //   5) the rest return pending
                //
                // Without this `self.trip()` those will never be awoken. With
                // the call to self.trip(), those that made it to poll() in 3)
                // will be awoken by `notify_waiters()`. For those the didn't,
                // one will be awoken by `notify_one()`, which will in-turn call
                // self.trip(), awaking more until there are no more to awake.
                self.trip();
                self.event = None;
                return Poll::Ready(());
            }
        }

        Poll::Pending
    }
}

impl TripWire {
    pub fn new() -> Self {
        TripWire {
            state: Arc::new(State {
                tripped: AtomicBool::new(false),
                notify: Notify::new()
            }),
            event: None,
        }
    }

    pub fn trip(&self) {
        self.tripped.store(true, Ordering::Release);
        self.notify.notify_waiters();
        self.notify.notify_one();
    }

    #[inline(always)]
    pub fn tripped(&self) -> bool {
        self.tripped.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::TripWire;

    #[test]
    fn ensure_is_send_sync_clone_unpin() {
        fn is_send_sync_clone_unpin<T: Send + Sync + Clone + Unpin>() {}
        is_send_sync_clone_unpin::<TripWire>();
    }

    #[tokio::test]
    async fn simple_trip() {
        let wire = TripWire::new();
        wire.trip();
        wire.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_trip() {
        use tokio::time::{sleep, Duration};
        use futures::stream::{FuturesUnordered as Set, StreamExt};
        use futures::future::{BoxFuture, FutureExt};

        let wire = TripWire::new();
        let mut futs: Set<BoxFuture<'static, bool>> = Set::new();
        for _ in 0..10 {
            futs.push(Box::pin(wire.clone().map(|_| false)));
        }

        let sleep = sleep(Duration::from_secs(1));
        futs.push(Box::pin(sleep.map(|_| true)));
        assert!(futs.next().await.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn general_trip() {
        let wire = TripWire::new();
        let mut tasks = vec![];
        for _ in 0..1000 {
            tasks.push(tokio::spawn(wire.clone()));
            tokio::task::yield_now().await;
        }

        wire.trip();
        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn single_stage_trip() {
        let mut tasks = vec![];
        for i in 0..1000 {
            // Trip once every 100. 50 will be left "untripped", but should be.
            if i % 2 == 0 {
                let wire = TripWire::new();
                tasks.push(tokio::spawn(wire.clone()));
                tasks.push(tokio::spawn(async move { wire.trip() }));
            } else {
                let wire = TripWire::new();
                let wire2 = wire.clone();
                tasks.push(tokio::spawn(async move { wire.trip() }));
                tasks.push(tokio::spawn(wire2));
            }
        }

        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn staged_trip() {
        let wire = TripWire::new();
        let mut tasks = vec![];
        for i in 0..1050 {
            let wire = wire.clone();
            // Trip once every 100. 50 will be left "untripped", but should be.
            let task = if i % 100 == 0 {
                tokio::spawn(async move { wire.trip() })
            } else {
                tokio::spawn(wire)
            };

            if i % 20 == 0 {
                tokio::task::yield_now().await;
            }

            tasks.push(task);
        }

        for task in tasks {
            task.await.unwrap();
        }
    }
}
