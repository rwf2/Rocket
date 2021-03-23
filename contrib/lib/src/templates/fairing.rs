use std::error::Error;

use crate::templates::{DEFAULT_TEMPLATE_DIR, Context, Engines};

use rocket::Rocket;
use rocket::fairing::{Fairing, Info, Kind};

pub(crate) use self::context::ContextManager;

type Callback = Box<dyn Fn(&mut Engines) -> Result<(), Box<dyn Error>>+ Send + Sync + 'static>;

#[cfg(not(debug_assertions))]
mod context {
    use std::ops::{Deref, DerefMut};
    use crate::templates::Context;
    use std::sync::RwLock;

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub(crate) struct ContextManager(RwLock<Context>);

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            ContextManager(RwLock::new(ctxt))
        }

        pub fn context(&self) -> impl Deref<Target=Context> + '_ {
            self.0.read().unwrap()
        }

        pub fn is_reloading(&self) -> bool {
            false
        }

        pub fn reload_templates(&self) {
            let root = self.context().root.clone();
            *self.context_mut() = Context::initialize(&root).unwrap();
        }

        fn context_mut(&self) -> impl DerefMut<Target=Context> + '_ {
            self.0.write().unwrap()
        }
    }
}

#[cfg(debug_assertions)]
mod context {
    use std::ops::{Deref, DerefMut};
    use std::sync::{RwLock, Mutex};
    use std::sync::mpsc::{channel, Receiver};

    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

    use super::{Callback, Context};

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub(crate) struct ContextManager {
        /// The current template context, inside an RwLock so it can be updated.
        context: RwLock<Context>,
        /// A filesystem watcher and the receive queue for its events.
        watcher: Option<(RecommendedWatcher, Mutex<Receiver<RawEvent>>)>,
    }

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            let (tx, rx) = channel();
            let watcher = raw_watcher(tx).and_then(|mut watcher| {
                watcher.watch(ctxt.root.canonicalize()?, RecursiveMode::Recursive)?;
                Ok(watcher)
            });

            let watcher = match watcher {
                Ok(watcher) => Some((watcher, Mutex::new(rx))),
                Err(e) => {
                    warn!("Failed to enable live template reloading: {}", e);
                    debug_!("Reload error: {:?}", e);
                    warn_!("Live template reloading is unavailable.");
                    None
                }
            };

            ContextManager { watcher, context: RwLock::new(ctxt), }
        }

        pub fn context(&self) -> impl Deref<Target=Context> + '_ {
            self.context.read().unwrap()
        }

        pub fn is_reloading(&self) -> bool {
            self.watcher.is_some()
        }

        pub fn reload_templates(&self) {
            let root = self.context().root.clone();
            *self.context_mut() = Context::initialize(&root).unwrap();
        }

        fn context_mut(&self) -> impl DerefMut<Target=Context> + '_ {
            self.context.write().unwrap()
        }

        /// Checks whether any template files have changed on disk. If there
        /// have been changes since the last reload, all templates are
        /// reinitialized from disk and the user's customization callback is run
        /// again.
        pub fn reload_if_needed(&self, callback: &Callback) {
            let templates_changes = self.watcher.as_ref()
                .map(|(_, rx)| rx.lock().expect("fsevents lock").try_iter().count() > 0);

            if let Some(true) = templates_changes {
                info_!("Change detected: reloading templates.");
                let root = self.context().root.clone();
                if let Some(mut new_ctxt) = Context::initialize(&root) {
                    match callback(&mut new_ctxt.engines) {
                        Ok(()) => *self.context_mut() = new_ctxt,
                        Err(e) => {
                            warn_!("The template customization callback returned an error:");
                            warn_!("{}", e);
                            warn_!("The existing templates will remain active.");
                        }
                    }
                } else {
                    warn_!("An error occurred while reloading templates.");
                    warn_!("The existing templates will remain active.");
                };
            }
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
    pub callback: Callback,
}

#[rocket::async_trait]
impl Fairing for TemplateFairing {
    fn info(&self) -> Info {
        // on_request only applies in debug mode, so only enable it in debug.
        #[cfg(debug_assertions)] let kind = Kind::Attach | Kind::Request;
        #[cfg(not(debug_assertions))] let kind = Kind::Attach;

        Info { kind, name: "Templates" }
    }

    /// Initializes the template context. Templates will be searched for in the
    /// `template_dir` config variable or the default ([DEFAULT_TEMPLATE_DIR]).
    /// The user's callback, if any was supplied, is called to customize the
    /// template engines. In debug mode, the `ContextManager::new` method
    /// initializes a directory watcher for auto-reloading of templates.
    async fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        use rocket::figment::{Source, value::magic::RelativePathBuf};

        let configured_dir = rocket.figment()
            .extract_inner::<RelativePathBuf>("template_dir")
            .map(|path| path.relative());

        let path = match configured_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_TEMPLATE_DIR.into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        match Context::initialize(&path) {
            Some(mut ctxt) => {
                use rocket::{logger::PaintExt, yansi::Paint};
                use crate::templates::Engines;

                info!("{}{}", Paint::emoji("📐 "), Paint::magenta("Templating:"));

                match (self.callback)(&mut ctxt.engines) {
                    Ok(()) => {
                        info_!("directory: {}", Paint::white(Source::from(&*path)));
                        info_!("engines: {:?}", Paint::white(Engines::ENABLED_EXTENSIONS));
                        Ok(rocket.manage(ContextManager::new(ctxt)))
                    }
                    Err(e) => {
                        error_!("The template customization callback returned an error:");
                        error_!("{}", e);
                        Err(rocket)
                    }
                }
            }
            None => {
                error_!("Launch will be aborted due to failed template initialization.");
                Err(rocket)
            }
        }
    }

    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data) {
        let cm = req.managed_state::<ContextManager>()
            .expect("Template ContextManager registered in on_attach");

        cm.reload_if_needed(&self.callback);
    }
}
