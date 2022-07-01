use crate::{DEFAULT_TEMPLATE_DIR, Context, Engines};
use crate::context::{Callback, ContextManager};

use rocket::{Rocket, Build, Orbit};
use rocket::fairing::{self, Fairing, Info, Kind};

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
        let kind = Kind::Ignite | Kind::Liftoff;
        #[cfg(debug_assertions)] let kind = kind | Kind::Request;

        Info { kind, name: "Templating" }
    }

    /// Initializes the template context. Templates will be searched for in the
    /// `template_dir` config variable or the default ([DEFAULT_TEMPLATE_DIR]).
    /// The user's callback, if any was supplied, is called to customize the
    /// template engines. In debug mode, the `ContextManager::new` method
    /// initializes a directory watcher for auto-reloading of templates.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        use rocket::figment::value::magic::RelativePathBuf;

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

        if let Some(ctxt) = Context::initialize(&path, &self.callback) {
            Ok(rocket.manage(ContextManager::new(ctxt)))
        } else {
            error_!("Template initialization failed. Aborting launch.");
            Err(rocket)
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        use rocket::{figment::Source, log::PaintExt, yansi::Paint};

        let cm = rocket.state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        info!("{}{}:", Paint::emoji("📐 "), Paint::magenta("Templating"));
        info_!("directory: {}", Paint::white(Source::from(&*cm.context().root)));
        info_!("engines: {:?}", Paint::white(Engines::ENABLED_EXTENSIONS));
    }

    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        let cm = req.rocket().state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        cm.reload_if_needed(&self.callback);
    }

}
