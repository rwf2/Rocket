use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use rocket::http::uncased::UncasedStr;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response, Rocket};

use crate::helmet::*;

/// A [`Fairing`](../../rocket/fairing/trait.Fairing.html) that adds HTTP
/// headers to outgoing responses that control security features on the browser.
///
/// # Usage
///
/// To use `SpaceHelmet`, first construct an instance of it. To use the default
/// set of headers, construct with [`SpaceHelmet::default()`](#method.default).
/// For an instance with no preset headers, use [`SpaceHelmet::new()`]. To
/// enable an additional header, use [`enable()`](SpaceHelmet::enable()), and to
/// disable a header, use [`disable()`](SpaceHelmet::disable()):
///
/// ```rust
/// use rocket_contrib::helmet::SpaceHelmet;
/// use rocket_contrib::helmet::{XssFilter, ExpectCt};
///
/// // A `SpaceHelmet` with the default headers:
/// let helmet = SpaceHelmet::default();
///
/// // A `SpaceHelmet` with the default headers minus `XssFilter`:
/// let helmet = SpaceHelmet::default().disable::<XssFilter>();
///
/// // A `SpaceHelmet` with the default headers plus `ExpectCt`.
/// let helmet = SpaceHelmet::default().enable(ExpectCt::default());
///
/// // A `SpaceHelmet` with only `XssFilter` and `ExpectCt`.
/// let helmet = SpaceHelmet::default()
///     .enable(XssFilter::default())
///     .enable(ExpectCt::default());
/// ```
///
/// Then, attach the instance of `SpaceHelmet` to your application's instance of
/// `Rocket`:
///
/// ```rust
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// # use rocket_contrib::helmet::SpaceHelmet;
/// # let helmet = SpaceHelmet::default();
/// rocket::ignite()
///     // ...
///     .attach(helmet)
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
/// non-development environment (e.g., staging or production), HSTS is
/// automatically enabled with its default policy and a warning is issued.
///
/// To get rid of this warning, explicitly [`enable()`](SpaceHelmet::enable())
/// an [`Hsts`] policy.
pub struct SpaceHelmet {
    policies: HashMap<&'static UncasedStr, Box<dyn SubPolicy>>,
    force_hsts: AtomicBool,
}

impl Default for SpaceHelmet {
    /// Returns a new `SpaceHelmet` instance. See the [table] for a description
    /// of the policies used by default.
    ///
    /// [table]: ./#supported-headers
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # extern crate rocket_contrib;
    /// use rocket_contrib::helmet::SpaceHelmet;
    ///
    /// let helmet = SpaceHelmet::default();
    /// ```
    fn default() -> Self {
        SpaceHelmet::new()
            .enable(NoSniff::default())
            .enable(Frame::default())
            .enable(XssFilter::default())
    }
}

impl SpaceHelmet {
    /// Returns an instance of `SpaceHelmet` with no headers enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket_contrib::helmet::SpaceHelmet;
    ///
    /// let helmet = SpaceHelmet::new();
    /// ```
    pub fn new() -> Self {
        SpaceHelmet {
            policies: HashMap::new(),
            force_hsts: AtomicBool::new(false),
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
    /// use rocket_contrib::helmet::SpaceHelmet;
    /// use rocket_contrib::helmet::NoSniff;
    ///
    /// let helmet = SpaceHelmet::new().enable(NoSniff::default());
    /// ```
    pub fn enable<P: Policy>(mut self, policy: P) -> Self {
        self.policies.insert(P::NAME.into(), Box::new(policy));
        self
    }

    /// Disables the policy header `policy`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket_contrib::helmet::SpaceHelmet;
    /// use rocket_contrib::helmet::NoSniff;
    ///
    /// let helmet = SpaceHelmet::default().disable::<NoSniff>();
    /// ```
    pub fn disable<P: Policy>(mut self) -> Self {
        self.policies.remove(UncasedStr::new(P::NAME));
        self
    }

    /// Returns `true` if the policy `P` is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket_contrib::helmet::SpaceHelmet;
    /// use rocket_contrib::helmet::{XssFilter, NoSniff, Frame};
    /// use rocket_contrib::helmet::{Hsts, ExpectCt, Referrer};
    ///
    /// let helmet = SpaceHelmet::default();
    ///
    /// assert!(helmet.is_enabled::<XssFilter>());
    /// assert!(helmet.is_enabled::<NoSniff>());
    /// assert!(helmet.is_enabled::<Frame>());
    ///
    /// assert!(!helmet.is_enabled::<Hsts>());
    /// assert!(!helmet.is_enabled::<ExpectCt>());
    /// assert!(!helmet.is_enabled::<Referrer>());
    /// ```
    pub fn is_enabled<P: Policy>(&self) -> bool {
        self.policies.contains_key(UncasedStr::new(P::NAME))
    }

    /// Sets all of the headers in `self.policies` in `response` as long as the
    /// header is not already in the response.
    fn apply(&self, response: &mut Response<'_>) {
        for policy in self.policies.values() {
            let name = policy.name();
            if response.headers().contains(name.as_str()) {
                warn!("Space Helmet: response contains a '{}' header.", name);
                warn_!("Refusing to overwrite existing header.");
                continue
            }

            // FIXME: Cache the rendered header.
            response.set_header(policy.header());
        }

        if self.force_hsts.load(Ordering::Relaxed) {
            if !response.headers().contains(Hsts::NAME) {
                response.set_header(&Hsts::default());
            }
        }
    }
}

impl Fairing for SpaceHelmet {
    fn info(&self) -> Info {
        Info {
            name: "Space Helmet",
            kind: Kind::Response | Kind::Launch,
        }
    }

    fn on_response(&self, _request: &Request<'_>, response: &mut Response<'_>) {
        self.apply(response);
    }

    fn on_launch(&self, rocket: &Rocket) {
        if rocket.config().tls_enabled()
            && !rocket.config().environment.is_dev()
            && !self.is_enabled::<Hsts>()
        {
            warn_!("Space Helmet: deploying with TLS without enabling HSTS.");
            warn_!("Enabling default HSTS policy.");
            info_!("To disable this warning, configure an HSTS policy.");
            self.force_hsts.store(true, Ordering::Relaxed);
        }
    }
}
