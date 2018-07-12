use super::DEFAULT_TEMPLATE_DIR;
use super::context::Context;
use super::engine::Engines;

use rocket::Rocket;
use rocket::config::ConfigError;
use rocket::fairing::{Fairing, Info, Kind};

#[cfg(not(debug_assertions))]
mod context {
    use std::ops::Deref;
    use super::Context;

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub struct ContextManager(Context);

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            ContextManager(ctxt)
        }

        pub fn get<'a>(&'a self) -> impl Deref<Target=Context> + 'a {
            &self.0
        }
    }
}

#[cfg(debug_assertions)]
mod context {
    extern crate notify;

    use std::ops::{Deref, DerefMut};
    use std::sync::{RwLock, Mutex};
    use std::sync::mpsc::{channel, Receiver};

    use super::{Context, Engines};

    use self::notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub struct ContextManager {
        /// The current template context, inside an RwLock so it can be updated
        context: RwLock<Context>,
        /// A filesystem watcher and the receive queue for its events
        watcher: Option<(RecommendedWatcher, Mutex<Receiver<RawEvent>>)>,
    }

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            let (tx, rx) = channel();

            let watcher = if let Ok(mut watcher) = raw_watcher(tx) {
                if watcher.watch(ctxt.root.clone(), RecursiveMode::Recursive).is_ok() {
                    Some((watcher, Mutex::new(rx)))
                } else {
                    warn!("Could not monitor the templates directory for changes.");
                    warn!("Live template reload will be unavailable");
                    None
                }
            } else {
                warn!("Could not instantiate a filesystem watcher.");
                warn!("Live template reload will be unavailable");
                None
            };

            ContextManager {
                watcher,
                context: RwLock::new(ctxt),
            }
        }

        pub fn get<'a>(&'a self) -> impl Deref<Target=Context> + 'a {
            self.context.read().unwrap()
        }

        fn get_mut<'a>(&'a self) -> impl DerefMut<Target=Context> + 'a {
            self.context.write().unwrap()
        }

        pub fn reload_if_needed<F: Fn(&mut Engines)>(&self, custom_callback: F) {
            self.watcher.as_ref().map(|w| {
                let rx = w.1.lock().expect("receive queue");
                let mut changed = false;
                while let Ok(_) = rx.try_recv() {
                    changed = true;
                }

                if changed {
                    warn!("Change detected, reloading templates");
                    let mut ctxt = self.get_mut();
                    if let Some(mut new_ctxt) = Context::initialize(ctxt.root.clone()) {
                        custom_callback(&mut new_ctxt.engines);
                        *ctxt = new_ctxt;
                    } else {
                        warn!("An error occurred while reloading templates.");
                        warn!("The previous templates will remain active.");
                    };
                }
            });
        }
    }
}

pub use self::context::ContextManager;

pub struct TemplateFairing {
    custom_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>,
}

impl TemplateFairing {
    pub fn new(custom_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>) -> TemplateFairing {
        TemplateFairing { custom_callback }
    }
}

impl Fairing for TemplateFairing {
    fn info(&self) -> Info {
        Info {
            name: "Templates",
            #[cfg(debug_assertions)]
            kind: Kind::Attach | Kind::Request,
            #[cfg(not(debug_assertions))]
            kind: Kind::Attach,
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        let mut template_root = rocket.config().root_relative(DEFAULT_TEMPLATE_DIR);
        match rocket.config().get_str("template_dir") {
            Ok(dir) => template_root = rocket.config().root_relative(dir),
            Err(ConfigError::NotFound) => { /* ignore missing configs */ }
            Err(e) => {
                e.pretty_print();
                warn_!("Using default templates directory '{:?}'", template_root);
            }
        };

        match Context::initialize(template_root.clone()) {
            Some(mut ctxt) => {
                (self.custom_callback)(&mut ctxt.engines);
                Ok(rocket.manage(ContextManager::new(ctxt)))
            }
            None => Err(rocket),
        }
    }

    #[cfg(debug_assertions)]
    fn on_request(&self, req: &mut ::rocket::Request, _data: &::rocket::Data) {
        let cm = req.guard::<::rocket::State<ContextManager>>().unwrap();
        cm.reload_if_needed(&*self.custom_callback);
    }
}
