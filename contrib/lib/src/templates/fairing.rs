use super::DEFAULT_TEMPLATE_DIR;
use super::context::Context;
use super::engine::Engines;

#[cfg(debug_assertions)]
use super::watch::TemplateWatcher;

use rocket::{Data, Request, Rocket};
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
    use std::ops::{Deref, DerefMut};
    use std::sync::RwLock;
    use super::{Context, Engines, TemplateWatcher};

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub struct ContextManager{ context: RwLock<Context>, watcher: Option<TemplateWatcher> }

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            ContextManager {
                watcher: TemplateWatcher::new(ctxt.root.clone()),
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
            if let Some(true) = self.watcher.as_ref().map(TemplateWatcher::needs_reload) {
                warn!("Change detected, reloading templates");
                let mut ctxt = self.get_mut();
                if let Some(mut new_ctxt) = Context::initialize(ctxt.root.clone()) {
                    custom_callback(&mut new_ctxt.engines);
                    *ctxt = new_ctxt;
                } else {
                    warn!("An error occurred while reloading templates. The previous templates will remain active.");
                };
            }
        }
    }
}

pub use self::context::ContextManager;

pub struct TemplateFairing {
    custom_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>,
}

impl TemplateFairing {
    pub fn new(custom_callback: Box<Fn(&mut Engines) + Send + Sync + 'static>) -> TemplateFairing
    {
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
    fn on_request(&self, req: &mut Request, _data: &Data) {
        use rocket::State;
        let cm = req.guard::<State<ContextManager>>().unwrap();
        cm.reload_if_needed(&*self.custom_callback);
    }
}
