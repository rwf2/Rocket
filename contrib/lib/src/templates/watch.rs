extern crate notify;

use self::notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;
use std::time::Duration;

pub struct TemplateWatcher {
    _watcher: RecommendedWatcher,
    recv_queue: Mutex<Receiver<DebouncedEvent>>,
}

impl TemplateWatcher {
    pub fn new<P: AsRef<Path>>(template_root: P) -> Option<TemplateWatcher> {
        let (tx, rx) = channel();
        match watcher(tx, Duration::from_secs(1)) {
            Ok(mut watcher) => match watcher.watch(template_root, RecursiveMode::Recursive) {
                Ok(_) => Some(TemplateWatcher { _watcher: watcher, recv_queue: Mutex::new(rx) }),
                Err(_) => {
                    warn!("Templates directory does not exist. Live template reload will be unavailable");
                    None
                }
            },
            Err(_) => {
                warn!("Could not instantiate a filesystem watcher. Live template reload will be unavailable");
                None
            },
        }
    }

    pub fn needs_reload(&self) -> bool {
        let rx = self.recv_queue.lock().expect("receive queue");
        let mut changed = false;
        while let Ok(_) = rx.try_recv() {
            changed = true;
        }
        changed
    }
}
