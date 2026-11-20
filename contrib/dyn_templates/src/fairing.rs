use rocket::{Rocket, Build, Orbit};
use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::figment::{Source, value::magic::RelativePathBuf};
use rocket::trace::Trace;

use crate::context::{Callback, Context, ContextManager};
use crate::template::DEFAULT_TEMPLATE_DIR;
use crate::engine::Engines;

/// A fairing that initializes and maintains templating state.
///
/// # Usage
///
/// To use `Templating`, first construct an instance with [`Templating::new()`]
/// or [`Templating::init()`]. The former uses the default configuration, while
/// the latter allows you to customize the templating engines. Then, attach the
/// instance to your application's instance of `Rocket`:
///
/// ```rust
/// use rocket_dyn_templates::Templating;
///
/// rocket::build()
///     // ...
///     .attach(Templating::new())
/// # ;
/// ```
///
/// This fairing _must_ be attached to any `Rocket` instance that wishes to
/// render templates. Failure to attach this fairing will result in an
/// "Uninitialized template context: missing fairing." error message when a
/// template is attempted to be rendered.
///
/// # Customization
///
/// Templating engines can be customized via two callbacks: one set via
/// [`Templating::init()`] or [`on_init()`](Self::on_init()), and another set
/// via [`on_load()`](Self::on_load()). Each is passed an [`Engines`], which
/// exposes the enabled engines. Regardless of the order in which the callbacks
/// are set, the fairing initializes templating as follows:
///
///   1. Creates the templating engines with Rocket's defaults.
///   2. Calls the `on_init` callback.
///   3. Loads the templates in `template_dir`.
///   4. Calls the `on_load` callback.
///
/// Which callback to use depends on the templates in `template_dir`. Some
/// engines check a template as soon as it's added, requiring everything it
/// uses, such as filters and other templates, to already exist. As such,
/// anything that templates in `template_dir` use must be registered in
/// `on_init`, before those templates are loaded. Conversely, a template that
/// uses templates in `template_dir` must be added in `on_load`, after those
/// templates are loaded. In summary:
///
/// | To...                                                | Use       |
/// |------------------------------------------------------|-----------|
/// | register helpers, filters, functions, or tests       | `on_init` |
/// | configure syntax, escaping, or other engine settings | `on_init` |
/// | add a template used by templates in `template_dir`   | `on_init` |
/// | add a template that uses templates in `template_dir` | `on_load` |
///
/// As a general rule of thumb, use `on_init`. Only use `on_load` to add
/// templates that extend, include, or otherwise use templates in
/// `template_dir`. See [`Engines`] for details on when each engine checks
/// templates.
///
/// For example, the following adds two Tera templates: `footer.html`, which
/// templates in `template_dir` can include, and `child.html`, which extends
/// `base.html`, a template in `template_dir`:
///
/// ```rust
/// # #[cfg(feature = "tera")] {
/// use rocket_dyn_templates::Templating;
///
/// let templating = Templating::try_init(|engines| {
///     engines.tera.add_raw_template("footer.html", "<footer>Rocket</footer>")?;
///     Ok(())
/// }).try_on_load(|engines| {
///     engines.tera.add_raw_template("child.html", concat!(
///         "{% extends \"base.html\" %}",
///         "{% block content %}Hello, world!{% endblock %}",
///     ))?;
///
///     Ok(())
/// });
///
/// rocket::build().attach(templating);
/// # }
/// ```
///
/// # Reloading
///
/// In debug mode, the fairing checks for modifications to templates before
/// every request and reloads them if necessary. Reloading repeats all of the
/// steps above with new engines, so both callbacks are called again.
///
/// # Errors
///
/// If loading templates or a callback fails, the remaining steps are skipped.
/// During ignition, this aborts launch. During reloading, the new engines are
/// discarded, and the previous templates continue to be used.
pub struct Templating {
    init: Option<Callback>,
    load: Option<Callback>,
}

impl Templating {
    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// If you wish to customize the internal templating engines, use
    /// [`Templating::init()`] instead.
    ///
    /// # Example
    ///
    /// To attach this fairing, simply call `attach` on the application's
    /// `Rocket` instance with `Templating::new()`:
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Templating;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Templating::new())
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn new() -> Self {
        Templating { init: None, load: None }
    }

    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// Unlike [`Templating::new()`], this method allows you to customize the
    /// templating engines via the function `f`, which is called before
    /// templates are loaded. This is equivalent to
    /// `Templating::new().on_init(f)`. See
    /// [customization](Templating#customization) for details.
    ///
    /// This method does not allow the function `f` to fail. If `f` is fallible,
    /// use [`Templating::try_init()`] instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Templating;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Templating::init(|engines| {
    ///             // engines.handlebars.register_helper ...
    ///         }))
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn init<F: Send + Sync + 'static>(f: F) -> Self
        where F: Fn(&mut Engines)
    {
        Self::new().on_init(f)
    }

    /// Returns a fairing that initializes and maintains templating state.
    ///
    /// This variant of [`Templating::init()`] allows a fallible `f` and is
    /// equivalent to `Templating::new().try_on_init(f)`. See
    /// [errors](Templating#errors) for what happens when `f` fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate rocket;
    /// extern crate rocket_dyn_templates;
    ///
    /// use rocket_dyn_templates::Templating;
    ///
    /// fn main() {
    ///     rocket::build()
    ///         // ...
    ///         .attach(Templating::try_init(|engines| {
    ///             // engines.handlebars.register_helper ...
    ///             Ok(())
    ///         }))
    ///         // ...
    ///     # ;
    /// }
    /// ```
    pub fn try_init<F: Send + Sync + 'static>(f: F) -> Self
        where F: Fn(&mut Engines) -> Result<(), Box<dyn std::error::Error>>
    {
        Self::new().try_on_init(f)
    }

    /// Sets the function `f` to be called before templates are loaded.
    ///
    /// Use this method to register helpers, configure engines, and add
    /// templates that templates in `template_dir` use. See
    /// [customization](Templating#customization) for details. If an `on_init`
    /// callback was previously set, it is replaced with `f`.
    ///
    /// This method does not allow the function `f` to fail. If `f` is fallible,
    /// use [`Templating::try_on_init()`] instead.
    pub fn on_init<F: Send + Sync + 'static>(self, f: F) -> Self
        where F: Fn(&mut Engines)
    {
        self.try_on_init(move |engines| { f(engines); Ok(()) })
    }

    /// Sets a fallible function `f` to be called before templates are loaded.
    ///
    /// This variant of [`Templating::on_init()`] allows a fallible `f`. See
    /// [errors](Templating#errors) for what happens when `f` fails.
    pub fn try_on_init<F: Send + Sync + 'static>(mut self, f: F) -> Self
        where F: Fn(&mut Engines) -> Result<(), Box<dyn std::error::Error>>
    {
        self.init = Some(Box::new(f));
        self
    }

    /// Sets the function `f` to be called after templates are loaded.
    ///
    /// Use this method to add templates that extend, include, or otherwise use
    /// templates in `template_dir`. See
    /// [customization](Templating#customization) for details. If an `on_load`
    /// callback was previously set, it is replaced with `f`.
    ///
    /// This method does not allow the function `f` to fail. If `f` is fallible,
    /// use [`Templating::try_on_load()`] instead.
    pub fn on_load<F: Send + Sync + 'static>(self, f: F) -> Self
        where F: Fn(&mut Engines)
    {
        self.try_on_load(move |engines| { f(engines); Ok(()) })
    }

    /// Sets a fallible function `f` to be called after templates are loaded.
    ///
    /// This variant of [`Templating::on_load()`] allows a fallible `f`. See
    /// [errors](Templating#errors) for what happens when `f` fails.
    pub fn try_on_load<F: Send + Sync + 'static>(mut self, f: F) -> Self
        where F: Fn(&mut Engines) -> Result<(), Box<dyn std::error::Error>>
    {
        self.load = Some(Box::new(f));
        self
    }
}

impl Default for Templating {
    fn default() -> Self {
        Self::new()
    }
}

#[rocket::async_trait]
impl Fairing for Templating {
    fn info(&self) -> Info {
        let kind = Kind::Ignite | Kind::Liftoff;
        #[cfg(debug_assertions)] let kind = kind | Kind::Request;

        Info { kind, name: "Templating" }
    }

    /// Initializes the template context. Templates will be searched for in the
    /// `template_dir` config variable or the default (`templates/`).
    /// The user's callbacks, if any were supplied, are called to customize the
    /// template engines. In debug mode, the `ContextManager::new` method
    /// initializes a directory watcher for auto-reloading of templates.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let configured_dir = rocket.figment()
            .extract_inner::<RelativePathBuf>("template_dir")
            .map(|path| path.relative());

        let path = match configured_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_TEMPLATE_DIR.into(),
            Err(e) => {
                e.trace_error();
                return Err(rocket);
            }
        };

        if let Some(ctxt) = Context::initialize(&path, self.init.as_ref(), self.load.as_ref()) {
            Ok(rocket.manage(ContextManager::new(ctxt)))
        } else {
            error!("Template initialization failed. Aborting launch.");
            Err(rocket)
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let cm = rocket.state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        span_info!("templating" => {
            info!(directory = %Source::from(&*cm.context().root));
            info!(engines = ?Engines::ENABLED_EXTENSIONS);
        });
    }

    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        let cm = req.rocket().state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        cm.reload_if_needed(self.init.as_ref(), self.load.as_ref());
    }
}
