use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::DEFAULT_TEMPLATE_DIR;
use super::context::Context;
use super::engine::Engines;

use rocket::{Data, Request, Rocket, State};
use rocket::config::ConfigError;
use rocket::fairing::{Fairing, Info, Kind};

pub struct ManagedContext(
    #[cfg(not(debug_assertions))]
    Context,
    #[cfg(debug_assertions)]
    RwLock<Context>
);

#[cfg(debug_assertions)]
impl ManagedContext {
    fn new(ctxt: Context) -> ManagedContext {
        ManagedContext(RwLock::new(ctxt))
    }

    pub fn get(&self) -> RwLockReadGuard<Context> {
        self.0.read().unwrap()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<Context> {
        self.0.write().unwrap()
    }
}

#[cfg(not(debug_assertions))]
impl ManagedContext {
    pub fn new(ctxt: Context) -> ManagedContext {
        ManagedContext(ctxt)
    }

    pub fn get(&self) -> &Context {
        &self.0
    }
}

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
