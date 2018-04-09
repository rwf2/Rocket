use std::sync::Mutex;

use super::DEFAULT_TEMPLATE_DIR;
use super::context::Context;
use super::engine::Engines;

use rocket::{Data, Request, Rocket, State};
use rocket::config::ConfigError;
use rocket::fairing::{Fairing, Info, Kind};

#[cfg(not(debug_assertions))]
mod context {
    use std::ops::Deref;
    use super::Context;

    /// Wraps a Context.
    /// This structure definition allows consuming code to use `get`
    /// regardless of whether or not debug mode is active, and enforces
    /// that `get_mut` can only be used in debug mode when the expensive
    /// interior mutability is active.
    pub struct ManagedContext(Context);

    impl ManagedContext {
        pub fn new(ctxt: Context) -> ManagedContext {
            ManagedContext(ctxt)
        }

        pub fn get(&self) -> impl Deref<Target=Context> {
            &self.0
        }
    }
}

#[cfg(debug_assertions)]
mod context {
    use std::ops::{Deref, DerefMut};
    use std::sync::RwLock;
    use super::Context;

    /// Wraps a Context, providing interior mutability in debug mode.
    /// This structure definition allows consuming code to use `get`
    /// regardless of whether or not debug mode is active, and enforces
    /// that `get_mut` can only be used in debug mode when the expensive
    /// interior mutability is active.
    pub struct ManagedContext(RwLock<Context>);

    impl ManagedContext {
        pub fn new(ctxt: Context) -> ManagedContext {
            ManagedContext(RwLock::new(ctxt))
        }

        pub fn get<'a>(&'a self) -> impl Deref<Target=Context> + 'a {
            self.0.read().unwrap()
        }

        pub fn get_mut<'a>(&'a self) -> impl DerefMut<Target=Context> + 'a {
            self.0.write().unwrap()
        }
    }
}

pub use self::context::ManagedContext;

pub struct TemplateFairing {
    custom_callback: Mutex<Option<Box<Fn(&mut Engines) + Send + Sync + 'static>>>,
}

impl TemplateFairing {
    pub fn new<F>(f: F) -> TemplateFairing
        where F: Fn(&mut Engines) + Send + Sync + 'static
    {
        TemplateFairing { custom_callback: Mutex::new(Some(Box::new(f))) }
    }
}

impl Fairing for TemplateFairing {
    fn info(&self) -> Info {
        Info {
            name: "Templates",
            kind: Kind::Attach | Kind::Request,
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

        let callback = self.custom_callback.lock().unwrap().take().expect("on_attach fairing called twice!");

        let ctxt = match Context::initialize(template_root, callback) {
            Some(ctxt) => ctxt,
            None => return Err(rocket),
        };

        Ok(rocket.manage(ManagedContext::new(ctxt)))
    }

    fn on_request(&self, req: &mut Request, _data: &Data) {
        #[cfg(debug_assertions)]
        {
            let mc = req.guard::<State<ManagedContext>>().succeeded().expect("context wrapper");
            mc.get_mut().reload_if_needed();
        }
    }
}
