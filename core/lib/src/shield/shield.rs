use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use state::Storage;
use yansi::Paint;

use crate::{Rocket, Request, Response, Orbit, Config};
use crate::fairing::{Fairing, Info, Kind};
use crate::http::{Header, uncased::UncasedStr};
use crate::log::PaintExt;
use crate::shield::*;

/// A [`Fairing`] that injects browser security and privacy headers into all
/// outgoing responses.
///
/// # Usage
///
/// To use `Shield`, first construct an instance of it. To use the default
/// set of headers, construct with [`Shield::default()`](#method.default).
/// For an instance with no preset headers, use [`Shield::new()`]. To
/// enable an additional header, use [`enable()`](Shield::enable()), and to
/// disable a header, use [`disable()`](Shield::disable()):
///
/// ```rust
/// use rocket::shield::Shield;
/// use rocket::shield::{XssFilter, ExpectCt};
///
/// // A `Shield` with the default headers:
/// let shield = Shield::default();
///
/// // A `Shield` with the default headers minus `XssFilter`:
/// let shield = Shield::default().disable::<XssFilter>();
///
/// // A `Shield` with the default headers plus `ExpectCt`.
/// let shield = Shield::default().enable(ExpectCt::default());
///
/// // A `Shield` with only `XssFilter` and `ExpectCt`.
/// let shield = Shield::default()
///     .enable(XssFilter::default())
///     .enable(ExpectCt::default());
/// ```
///
/// Then, attach the instance of `Shield` to your application's instance of
/// `Rocket`:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::shield::Shield;
/// # let shield = Shield::default();
/// rocket::build()
///     // ...
///     .attach(shield)
/// # ;
/// ```
///
/// The fairing will inject all enabled headers into all outgoing responses
/// _unless_ the response already contains a header with the same name. If it
/// does contain the header, a warning is emitted, and the header is not
/// overwritten.
///
/// # TLS and HSTS
///
/// If TLS is configured and enabled when the application is launched in a
/// non-debug profile, HSTS is automatically enabled with its default policy and
/// a warning is logged.
///
/// To get rid of this warning, explicitly [`Shield::enable()`] an [`Hsts`]
/// policy.
pub struct Shield {
    /// Enabled policies where the key is the header name.
    policies: HashMap<&'static UncasedStr, Box<dyn SubPolicy>>,
    /// Whether to enforce HSTS even though the user didn't enable it.
    force_hsts: AtomicBool,
    /// Headers pre-rendered at liftoff from the configured policies.
    rendered: Storage<Vec<Header<'static>>>,
}

impl Default for Shield {
    /// Returns a new `Shield` instance. See the [table] for a description
    /// of the policies used by default.
    ///
    /// [table]: ./#supported-headers
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::shield::Shield;
    ///
    /// let shield = Shield::default();
    /// ```
    fn default() -> Self {
        Shield::new()
            .enable(NoSniff::default())
            .enable(Frame::default())
            .enable(Permission::default())
    }
}

impl Shield {
    /// Returns an instance of `Shield` with no headers enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::shield::Shield;
    ///
    /// let shield = Shield::new();
    /// ```
    pub fn new() -> Self {
        Shield {
            policies: HashMap::new(),
            force_hsts: AtomicBool::new(false),
            rendered: Storage::new(),
        }
    }

    /// Enables the policy header `policy`.
    ///
    /// If the poliicy was previously enabled, the configuration is replaced
    /// with that of `policy`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::shield::Shield;
    /// use rocket::shield::NoSniff;
    ///
    /// let shield = Shield::new().enable(NoSniff::default());
    /// ```
    pub fn enable<P: Policy>(mut self, policy: P) -> Self {
        self.rendered = Storage::new();
        self.policies.insert(P::NAME.into(), Box::new(policy));
        self
    }

    /// Disables the policy header `policy`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::shield::Shield;
    /// use rocket::shield::NoSniff;
    ///
    /// let shield = Shield::default().disable::<NoSniff>();
    /// ```
    pub fn disable<P: Policy>(mut self) -> Self {
        self.rendered = Storage::new();
        self.policies.remove(UncasedStr::new(P::NAME));
        self
    }

    /// Returns `true` if the policy `P` is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::shield::Shield;
    /// use rocket::shield::{Permission, NoSniff, Frame};
    /// use rocket::shield::{Prefetch, ExpectCt, Referrer};
    ///
    /// let shield = Shield::default();
    ///
    /// assert!(shield.is_enabled::<NoSniff>());
    /// assert!(shield.is_enabled::<Frame>());
    /// assert!(shield.is_enabled::<Permission>());
    ///
    /// assert!(!shield.is_enabled::<Prefetch>());
    /// assert!(!shield.is_enabled::<ExpectCt>());
    /// assert!(!shield.is_enabled::<Referrer>());
    /// ```
    pub fn is_enabled<P: Policy>(&self) -> bool {
        self.policies.contains_key(UncasedStr::new(P::NAME))
    }

    fn headers(&self) -> &[Header<'static>] {
        self.rendered.get_or_set(|| {
            let mut headers: Vec<_> = self.policies.values()
                .map(|p| p.header())
                .collect();

            if self.force_hsts.load(Ordering::Acquire) {
                headers.push(Policy::header(&Hsts::default()));
            }

            headers
        })
    }
}

#[crate::async_trait]
impl Fairing for Shield {
    fn info(&self) -> Info {
        Info {
            name: "Shield",
            kind: Kind::Liftoff | Kind::Response | Kind::Singleton,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let force_hsts = rocket.config().tls_enabled()
            && rocket.figment().profile() != Config::DEBUG_PROFILE
            && !self.is_enabled::<Hsts>();

        if force_hsts {
            self.force_hsts.store(true, Ordering::Release);
        }

        if !self.headers().is_empty() {
            info!("{}{}:", Paint::emoji("🛡️ "), Paint::magenta("Shield"));

            for header in self.headers() {
                info_!("{}: {}", header.name(), Paint::default(header.value()));
            }

            if force_hsts {
                warn_!("Detected TLS-enabled liftoff without enabling HSTS.");
                warn_!("Shield has enabled a default HSTS policy.");
                info_!("To remove this warning, configure an HSTS policy.");
            }
        }
    }

    async fn on_response<'r>(&self, _: &'r Request<'_>, response: &mut Response<'r>) {
        // Set all of the headers in `self.policies` in `response` as long as
        // the header is not already in the response.
        for header in self.headers() {
            if response.headers().contains(header.name()) {
                warn!("Shield: response contains a '{}' header.", header.name());
                warn_!("Refusing to overwrite existing header.");
                continue
            }

            response.set_header(header.clone());
        }
    }
}
