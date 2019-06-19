use crate::templates::{DEFAULT_TEMPLATE_DIR, Context, Engines};

use rocket::Rocket;
use rocket::config::ConfigError;
use rocket::fairing::{Fairing, Info, Kind};

crate use self::context::ContextManager;

#[cfg(not(debug_assertions))]
mod context {
    use std::ops::Deref;
    use crate::templates::Context;

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    crate struct ContextManager(Context);

    impl ContextManager {
        crate fn new(ctxt: Context) -> ContextManager {
            ContextManager(ctxt)
        }

        crate fn context<'a>(&'a self) -> impl Deref<Target=Context> + 'a {
            &self.0
        }

        crate fn is_reloading(&self) -> bool {
            false
        }
    }
}

#[cfg(debug_assertions)]
mod context {
    use std::ops::{Deref, DerefMut};
    use std::sync::{RwLock, Mutex};
    use std::sync::mpsc::{channel, Receiver};

    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

    use crate::templates::{Context, Engines};

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    crate struct ContextManager {
        /// The current template context, inside an RwLock so it can be updated.
        context: RwLock<Context>,
        /// A filesystem watcher and the receive queue for its events.
        watcher: Option<Mutex<(RecommendedWatcher, Receiver<RawEvent>)>>,
    }

    impl ContextManager {
        crate fn new(ctxt: Context) -> ContextManager {
            let (tx, rx) = channel();
            let watcher = raw_watcher(tx).and_then(|mut watcher| {
                watcher.watch(ctxt.root.canonicalize()?, RecursiveMode::Recursive)?;
                Ok(watcher)
            });

            let watcher = match watcher {
                Ok(watcher) => Some(Mutex::new((watcher, rx))),
                Err(e) => {
                    warn!("Failed to enable live template reloading: {}", e);
                    debug_!("Reload error: {:?}", e);
                    warn_!("Live template reloading is unavailable.");
                    None
                }
            };

            ContextManager {
                watcher,
                context: RwLock::new(ctxt),
            }
        }

        crate fn context(&self) -> impl Deref<Target=Context> + '_ {
            self.context.read().unwrap()
        }

        crate fn is_reloading(&self) -> bool {
            self.watcher.is_some()
        }

        fn context_mut(&self) -> impl DerefMut<Target=Context> + '_ {
            self.context.write().unwrap()
        }

        /// Checks whether any template files have changed on disk. If there
        /// have been changes since the last reload, all templates are
        /// reinitialized from disk and the user's customization callback is run
        /// again.
        crate fn reload_if_needed<F: Fn(&mut Engines)>(&self, custom_callback: F) {
            self.watcher.as_ref().map(|w| {
                let rx_lock = w.lock().expect("receive queue lock");
                let mut changed = false;
                while let Ok(_) = rx_lock.1.try_recv() {
                    changed = true;
                }

                if changed {
                    info_!("Change detected: reloading templates.");
                    let mut ctxt = self.context_mut();
                    if let Some(mut new_ctxt) = Context::initialize(ctxt.root.clone()) {
                        custom_callback(&mut new_ctxt.engines);
                        *ctxt = new_ctxt;
                    } else {
                        warn_!("An error occurred while reloading templates.");
                        warn_!("The previous templates will remain active.");
                    };
                }
            });
        }
    }
}

/// The TemplateFairing initializes the template system on attach, running
/// custom_callback after templates have been loaded. In debug mode, the fairing
/// checks for modifications to templates before every request and reloads them
/// if necessary.
pub struct TemplateFairing {
    /// The user-provided customization callback, allowing the use of
    /// functionality specific to individual template engines. In debug mode,
    /// this callback might be run multiple times as templates are reloaded.
    crate custom_callback: Box<dyn Fn(&mut Engines) + Send + Sync + 'static>,
}

impl Fairing for TemplateFairing {
    fn info(&self) -> Info {
        // The on_request part of this fairing only applies in debug
        // mode, so only register it in debug mode.
        Info {
            name: "Templates",
            #[cfg(debug_assertions)]
            kind: Kind::Attach | Kind::Request,
            #[cfg(not(debug_assertions))]
            kind: Kind::Attach,
        }
    }

    /// Initializes the template context. Templates will be searched for in the
    /// `template_dir` config variable or the default ([DEFAULT_TEMPLATE_DIR]).
    /// The user's callback, if any was supplied, is called to customize the
    /// template engines. In debug mode, the `ContextManager::new` method
    /// initializes a directory watcher for auto-reloading of templates.
    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        let mut template_root = rocket.config().root_relative(DEFAULT_TEMPLATE_DIR);
        match rocket.config().get_str("template_dir") {
            Ok(dir) => template_root = rocket.config().root_relative(dir),
            Err(ConfigError::Missing(_)) => { /* ignore missing */ }
            Err(e) => {
                e.pretty_print();
                warn_!("Using default templates directory '{:?}'", template_root);
            }
        };

        match Context::initialize(template_root) {
            Some(mut ctxt) => {
                (self.custom_callback)(&mut ctxt.engines);
                Ok(rocket.manage(ContextManager::new(ctxt)))
            }
            None => Err(rocket),
        }
    }

    #[cfg(debug_assertions)]
    fn on_request(&self, req: &mut rocket::Request<'_>, _data: &rocket::Data) {
        let cm = req.guard::<rocket::State<'_, ContextManager>>()
            .expect("Template ContextManager registered in on_attach");

        cm.reload_if_needed(&*self.custom_callback);
    }
}
