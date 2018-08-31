use std::sync::atomic::{AtomicBool, Ordering};

extern crate rocket;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::Rocket;
use rocket::{Request, Response};

use space_helmet::policy::*;

pub struct Helmet<'a> {
    expect_ct_policy: Option<ExpectCTPolicy<'a>>,
    no_sniff_policy: Option<NoSniffPolicy>,
    xss_protect_policy: Option<XSSPolicy<'a>>,
    frameguard_policy: Option<FramePolicy<'a>>,
    hsts_policy: Option<HSTSPolicy>,
    force_hsts_policy: Option<HSTSPolicy>,
    force_hsts: AtomicBool,
    referrer_policy: Option<ReferrerPolicy>,
}

//helper for Helmet.apply
macro_rules! apply_header {
    ($self:ident, $response:ident, $policy_name:ident) => {
        if let Some(ref policy) = $self.$policy_name {
            if $response.set_header(policy) {
                warn_!(
                    "Warning (Space Helmet), found existing header while trying to set {}",
                    Header::from(policy)
                );
            }
        }
    };
}

impl<'a> Helmet<'a> {
    ///Returns a new `Helmet` instance with `no_sniff`, `frameguard`, and `xss_protect` enabled by default.
    pub fn default() -> Self {
        Self {
            expect_ct_policy: None,
            no_sniff_policy: Some(NoSniffPolicy::default()),
            frameguard_policy: Some(FramePolicy::default()),
            xss_protect_policy: Some(XSSPolicy::default()),
            hsts_policy: None,
            force_hsts_policy: Some(HSTSPolicy::default()),
            force_hsts: AtomicBool::new(false),
            referrer_policy: None,
        }
    }

    ///Same as `Helmet::default()`.
    pub fn new() -> Self {
        Helmet::default()
    }

    ///Sets the [X-XSS-Protection](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection) header to the given `policy` or disables it if `policy == None`.
    pub fn xss_protect<T: Into<Option<XSSPolicy<'a>>>>(mut self, policy: T) -> Helmet<'a> {
        self.xss_protect_policy = policy.into();
        self
    }

    ///Sets the [X-Content-Type-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options) header to `policy` or disables it if `policy == None`.
    pub fn no_sniff<T: Into<Option<NoSniffPolicy>>>(mut self, policy: T) -> Helmet<'a> {
        self.no_sniff_policy = policy.into();
        self
    }

    ///Sets the [X-Frame-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options) header to `policy`, or disables it if `policy == None`.
    pub fn frameguard<T: Into<Option<FramePolicy<'a>>>>(mut self, policy: T) -> Helmet<'a> {
        self.frameguard_policy = policy.into();
        self
    }

    ///Sets the [Strict-Transport-Security](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security) header to `policy`, or disables it if `policy == None`.
    pub fn hsts<T: Into<Option<HSTSPolicy>>>(mut self, policy: T) -> Helmet<'a> {
        self.hsts_policy = policy.into();
        self
    }

    ///Sets the [Expect-CT](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT) header to `policy`, or disables it if `policy == None`.
    pub fn expect_ct<T: Into<Option<ExpectCTPolicy<'a>>>>(mut self, policy: T) -> Helmet<'a> {
        self.expect_ct_policy = policy.into();
        self
    }

    ///Sets the [Referrer-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy) header to `policy`, or disables it if `policy == None`.
    pub fn referrer_policy<T: Into<Option<ReferrerPolicy>>>(mut self, policy: T) -> Helmet<'a> {
        self.referrer_policy = policy.into();
        self
    }

    fn apply(&self, response: &mut rocket::Response) {
        apply_header!(self, response, no_sniff_policy);
        apply_header!(self, response, xss_protect_policy);
        apply_header!(self, response, frameguard_policy);
        apply_header!(self, response, expect_ct_policy);
        apply_header!(self, response, referrer_policy);
        if self.hsts_policy.is_some() {
            apply_header!(self, response, hsts_policy);
        } else {
            if self.force_hsts.load(Ordering::Relaxed) {
                apply_header!(self, response, force_hsts_policy);
            }
        }
    }
}

impl Fairing for Helmet<'static> {
    fn info(&self) -> Info {
        Info {
            name: "Rocket Helmet (HTTP Security Headers)",
            kind: Kind::Response | Kind::Launch,
        }
    }

    fn on_response(&self, _request: &Request, response: &mut Response) {
        self.apply(response);
    }

    fn on_launch(&self, rocket: &Rocket) {
        if rocket.config().tls_enabled()
            && !rocket.config().environment.is_dev()
            && !self.hsts_policy.is_some()
        {
            warn_!("Warning (Space Helmet): deploying with TLS without enabling hsts!!!");
            warn_!("--Forcing use of hsts with default policy--");
            warn_!(">>recommended fix: set an hsts policy for space helmet<<");
            self.force_hsts.store(true, Ordering::Relaxed);
        }
    }
}
