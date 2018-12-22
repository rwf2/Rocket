//! Module containing the [`Policy`] trait and types that implement it.

use std::borrow::Cow;

use rocket::http::{Header, uri::Uri, uncased::UncasedStr};

use helmet::time::Duration;

/// Trait implemented by security and privacy policy headers.
///
/// Types that implement this trait can be [`enable()`]d and [`disable()`]d on
/// instances of [`SpaceHelmet`].
///
/// [`SpaceHelmet`]: ::helmet::SpaceHelmet
/// [`enable()`]: ::helmet::SpaceHelmet::enable()
/// [`disable()`]: ::helmet::SpaceHelmet::disable()
pub trait Policy: Default + Send + Sync + 'static {
    /// The actual name of the HTTP header.
    ///
    /// This name must uniquely identify the header as it is used to determine
    /// whether two implementations of `Policy` are for the same header. Use the
    /// real HTTP header's name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # extern crate rocket_contrib;
    /// # use rocket::http::Header;
    /// use rocket_contrib::helmet::Policy;
    ///
    /// #[derive(Default)]
    /// struct MyPolicy;
    ///
    /// impl Policy for MyPolicy {
    ///     const NAME: &'static str = "X-My-Policy";
    /// #   fn header(&self) -> Header<'static> { unimplemented!() }
    /// }
    /// ```
    const NAME: &'static str;

    /// Returns the [`Header`](../../rocket/http/struct.Header.html) to attach
    /// to all outgoing responses.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # extern crate rocket_contrib;
    /// use rocket::http::Header;
    /// use rocket_contrib::helmet::Policy;
    ///
    /// #[derive(Default)]
    /// struct MyPolicy;
    ///
    /// impl Policy for MyPolicy {
    /// #   const NAME: &'static str = "X-My-Policy";
    ///     fn header(&self) -> Header<'static> {
    ///         Header::new(Self::NAME, "value-to-enable")
    ///     }
    /// }
    /// ```
    fn header(&self) -> Header<'static>;
}

crate trait SubPolicy: Send + Sync {
    fn name(&self) -> &'static UncasedStr;
    fn header(&self) -> Header<'static>;
}

impl<P: Policy> SubPolicy for P {
    fn name(&self) -> &'static UncasedStr {
        P::NAME.into()
    }

    fn header(&self) -> Header<'static> {
        Policy::header(self)
    }
}

macro_rules! impl_policy {
    ($T:ty, $name:expr) => (
        impl Policy for $T {
            const NAME: &'static str = $name;

            fn header(&self) -> Header<'static> {
                self.into()
            }
        }
    )
}

impl_policy!(XssFilter, "X-XSS-Protection");
impl_policy!(NoSniff, "X-Content-Type-Options");
impl_policy!(Frame, "X-Frame-Options");
impl_policy!(Hsts, "Strict-Transport-Security");
impl_policy!(ExpectCt, "Expect-CT");
impl_policy!(Referrer, "Referrer-Policy");

/// The [Referrer-Policy] header: controls the value set by the browser for the
/// [Referer] header.
///
/// Tells the browser if it should send all or part of URL of the current page
/// to the next site the user navigates to via the [Referer] header. This can be
/// important for security as the URL itself might expose sensitive data, such
/// as a hidden file path or personal identifier.
///
/// [Referrer-Policy]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy
/// [Referer]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referer
pub enum Referrer {
    /// Omits the `Referer` header (_SpaceHelmet default_).
    NoReferrer,

    /// Omits the `Referer` header on connection downgrade i.e. following HTTP
    /// link from HTTPS site (_Browser default_).
    NoReferrerWhenDowngrade,

    /// Only send the origin of part of the URL, e.g. the origin of
    /// https://foo.com/bob.html is https://foo.com
    Origin,

    /// Send full URL for same-origin requests, only send origin part when
    /// replying to [cross-origin] requests.
    ///
    /// [cross-origin]: https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS
    OriginWhenCrossOrigin,

    /// Send full URL for same-origin requests only.
    SameOrigin,

    /// Only send origin part of URL, only send if protocol security level
    /// remains the same e.g. HTTPS to HTTPS.
    StrictOrigin,

    /// Send full URL for same-origin requests. For cross-origin requests, only
    /// send origin part of URL if protocl security level remains the same e.g.
    /// HTTPS to HTTPS.
    StrictOriginWhenCrossOrigin,

    /// Send full URL for same-origin or cross-origin requests. _This will leak
    /// the full URL of TLS protected resources to insecure origins. Use with
    /// caution._
    UnsafeUrl,
 }

/// Defaults to [`Referrer::NoReferrer`]. Tells the browser to omit the
/// `Referer` header.
impl Default for Referrer {
    fn default() -> Referrer {
        Referrer::NoReferrer
    }
}

impl<'h, 'a> Into<Header<'h>> for &'a Referrer {
    fn into(self) -> Header<'h> {
        let policy_string = match self {
            Referrer::NoReferrer => "no-referrer",
            Referrer::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Referrer::Origin => "origin",
            Referrer::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Referrer::SameOrigin => "same-origin",
            Referrer::StrictOrigin => "strict-origin",
            Referrer::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Referrer::UnsafeUrl => "unsafe-url",
        };

        Header::new(Referrer::NAME, policy_string)
    }
}

/// The [Expect-CT] header: enables [Certificate Transparency] to detect and
/// prevent misuse of TLS certificates.
///
/// [Certificate Transparency] solves a variety of problems with public TLS/SSL
/// certificate management and is valuable measure for all public applications.
/// If you're just [getting started] with certificate transparency, ensure that
/// your [site is in compliance][getting started] before you enable enforcement
/// with [`ExpectCt::Enforce`] or [`ExpectCt::ReportAndEnforce`]. Failure to do
/// so will result in the browser refusing to communicate with your application.
/// _You have been warned_.
///
/// [Expect-CT]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT
/// [Certificate Transparency]: http://www.certificate-transparency.org/what-is-ct
/// [getting started]: http://www.certificate-transparency.org/getting-started
pub enum ExpectCt {
    /// Enforce certificate compliance for the next [`Duration`]. Ensure that
    /// your certificates are in compliance before turning on enforcement.
    /// (_SpaceHelmet_ default).
    Enforce(Duration),

    /// Report to `Uri`, but do not enforce, compliance violations for the next
    /// [`Duration`]. Doesn't provide any protection but is a good way make sure
    /// things are working correctly before turning on enforcement in
    /// production.
    Report(Duration, Uri<'static>),

    /// Enforce compliance and report violations to `Uri` for the next
    /// [`Duration`].
    ReportAndEnforce(Duration, Uri<'static>),
}

/// Defaults to [`ExpectCt::Enforce(Duration::days(30))`], enforce CT
/// compliance, see [draft] standard for more.
///
/// [draft]: https://tools.ietf.org/html/draft-ietf-httpbis-expect-ct-03#page-15
impl Default for ExpectCt {
    fn default() -> ExpectCt {
        ExpectCt::Enforce(Duration::days(30))
    }
}

impl<'a> Into<Header<'static>> for &'a ExpectCt {
    fn into(self) -> Header<'static> {
        let policy_string =  match self {
            ExpectCt::Enforce(age) => format!("max-age={}, enforce", age.num_seconds()),
            ExpectCt::Report(age, uri) => {
                format!(r#"max-age={}, report-uri="{}""#, age.num_seconds(), uri)
            }
            ExpectCt::ReportAndEnforce(age, uri) => {
                format!("max-age={}, enforce, report-uri=\"{}\"", age.num_seconds(), uri)
            }
        };

        Header::new(ExpectCt::NAME, policy_string)
    }
}

/// The [X-Content-Type-Options] header: turns off [mime sniffing] which can
/// prevent certain [attacks].
///
/// [mime sniffing]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types#MIME_sniffing
/// [X-Content-Type-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options
/// [attacks]: https://helmetjs.github.io/docs/dont-sniff-mimetype/
pub enum NoSniff {
    /// Turns off mime sniffing.
    Enable,
}

/// Defaults to [`NoSniff::Enable`], turns off mime sniffing.
impl Default for NoSniff {
    fn default() -> NoSniff {
        NoSniff::Enable
    }
}

impl<'h, 'a> Into<Header<'h>> for &'a NoSniff {
    fn into(self) -> Header<'h> {
        Header::new(NoSniff::NAME, "nosniff")
    }
}

/// The HTTP [Strict-Transport-Security] (HSTS) header: enforces strict HTTPS
/// usage.
///
/// HSTS tells the browser that the site should only be accessed using HTTPS
/// instead of HTTP. HSTS prevents a variety of downgrading attacks and should
/// always be used when TLS is enabled.  `SpaceHelmet` will turn HSTS on and
/// issue a warning if you enable TLS without enabling HSTS when the application
/// is run in the staging or production environments.
///
/// While HSTS is important for HTTPS security, incorrectly configured HSTS can
/// lead to problems as you are disallowing access to non-HTTPS enabled parts of
/// your site. [Yelp engineering] has good discussion of potential challenges
/// that can arise and how to roll this out in a large scale setting. So, if
/// you use TLS, use HSTS, but roll it out with care.
///
/// [TLS]: https://rocket.rs/guide/configuration/#configuring-tls
/// [Strict-Transport-Security]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
/// [default policy]: /rocket_contrib/helmet/enum.Hsts.html#impl-Default
/// [Yelp engineering]: https://engineeringblog.yelp.com/2017/09/the-road-to-hsts.html
/// [Staging]: /rocket/config/enum.Environment.html#variant.Staging
/// [Production]: /rocket/config/enum.Environment.html#variant.Production
pub enum Hsts {
    /// Browser should only permit this site to be accesses by HTTPS for the
    /// next [`Duration`].
    Enable(Duration),

    /// Like [`Hsts::Enable`], but also apply to all of the site's subdomains.
    IncludeSubDomains(Duration),

    /// Google maintains an [HSTS preload service] that can be used to prevent
    /// the browser from ever connecting to your site over an insecure
    /// connection. Read more [here]. Don't enable this before you have
    /// registered your site.
    ///
    /// [HSTS preload service]: https://hstspreload.org/
    /// [here]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security#Preloading_Strict_Transport_Security
    Preload(Duration),
}

/// Defaults to `Hsts::Enable(Duration::weeks(52))`.
impl Default for Hsts {
    fn default() -> Hsts {
        Hsts::Enable(Duration::weeks(52))
    }
}

impl<'a> Into<Header<'static>> for &'a Hsts {
    fn into(self) -> Header<'static> {
        let policy_string = match self {
            Hsts::Enable(age) => format!("max-age={}", age.num_seconds()),
            Hsts::IncludeSubDomains(age) => {
                format!("max-age={}; includeSubDomains", age.num_seconds())
            }
            Hsts::Preload(age) => format!("max-age={}; preload", age.num_seconds()),
        };

        Header::new(Hsts::NAME, policy_string)
    }
}

/// The [X-Frame-Options] header: helps prevent [clickjacking] attacks.
///
/// Controls whether the browser should allow the page to render in a `<frame>`,
/// [`<iframe>`][iframe] or `<object>`. This can be used to prevent
/// [clickjacking] attacks.
///
/// [X-Frame-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options
/// [clickjacking]: https://en.wikipedia.org/wiki/Clickjacking
/// [owasp-clickjacking]: https://www.owasp.org/index.php/Clickjacking_Defense_Cheat_Sheet
/// [iframe]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe
pub enum Frame {
    /// Page cannot be displayed in a frame.
    Deny,

    /// Page can only be displayed in a frame if the page trying to render it is
    /// in the same origin. Interpretation of same-origin is [browser
    /// dependent][X-Frame-Options].
    ///
    /// [X-Frame-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options
    SameOrigin,

    /// Page can only be displayed in a frame if the page trying to render it is
    /// in the origin for `Uri`. Interpretation of origin is [browser
    /// dependent][X-Frame-Options].
    ///
    /// [X-Frame-Options]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options
    AllowFrom(Uri<'static>),
}

/// Defaults to [`Frame::SameOrigin`].
impl Default for Frame {
    fn default() -> Frame {
        Frame::SameOrigin
    }
}

impl<'a> Into<Header<'static>> for &'a Frame {
    fn into(self) -> Header<'static> {
        let policy_string: Cow<'static, str> = match self {
            Frame::Deny => "DENY".into(),
            Frame::SameOrigin => "SAMEORIGIN".into(),
            Frame::AllowFrom(uri) => format!("ALLOW-FROM {}", uri).into(),
        };

        Header::new(Frame::NAME, policy_string)
    }
}

/// The [X-XSS-Protection] header: filters some forms of reflected [XSS]
/// attacks.
///
/// [X-XSS-Protection]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection
/// [XSS]: https://developer.mozilla.org/en-US/docs/Glossary/Cross-site_scripting
pub enum XssFilter {
    /// Disables XSS filtering.
    Disable,

    /// Enables XSS filtering. If XSS is detected, the browser will sanitize
    /// before rendering the page (_SpaceHelmet default_).
    Enable,

    /// Enables XSS filtering. If XSS is detected, the browser will not
    /// render the page.
    EnableBlock,

    /// Enables XSS filtering. If XSS is detected, the browser will sanitize and
    /// render the page and report the violation to the given `Uri`. (_Chromium
    /// only_)
    EnableReport(Uri<'static>),
}

/// Defaults to [`XssFilter::Enable`].
impl Default for XssFilter {
    fn default() -> XssFilter {
        XssFilter::Enable
    }
}

impl<'a> Into<Header<'static>> for &'a XssFilter {
    fn into(self) -> Header<'static> {
        let policy_string: Cow<'static, str> = match self {
            XssFilter::Disable => "0".into(),
            XssFilter::Enable => "1".into(),
            XssFilter::EnableBlock => "1; mode=block".into(),
            XssFilter::EnableReport(u) => format!("{}{}", "1; report=", u).into(),
        };

        Header::new(XssFilter::NAME, policy_string)
    }
}

/// The [Content-Security-Policy] header: specifies origins to protect against [XSS]
/// attacks.
///
/// [Content-Security-Policy]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Security-Policy
/// [XSS]: https://developer.mozilla.org/en-US/docs/Glossary/Cross-site_scripting
pub struct Csp {
    default_src: Vec<Cow<'static, str>>,
    style_src: Vec<Cow<'static, str>>,
    connect_src: Vec<Cow<'static, str>>,
    font_src: Vec<Cow<'static, str>>,
    frame_src: Vec<Cow<'static, str>>,
    img_src: Vec<Cow<'static, str>>,
    manifest_src: Vec<Cow<'static, str>>,
    media_src: Vec<Cow<'static, str>>,
    object_src: Vec<Cow<'static, str>>,
    script_src: Vec<Cow<'static, str>>,
}

impl Policy for Csp {
    const NAME: &'static str = "Content-Security-Policy";

    fn header(&self) -> Header<'static> {
        self.into()
    }
}

impl Csp {
    /// Only allow own Uri to load resource
    pub const SELF: &'static str = "'self'";

    /// Only allow inline resources
    pub const NONE: &'static str = "'none'";

    pub fn new() -> Csp {
        Csp {
            default_src: Vec::new(),
            style_src: Vec::new(),
            connect_src: Vec::new(),
            font_src: Vec::new(),
            frame_src: Vec::new(),
            img_src: Vec::new(),
            manifest_src: Vec::new(),
            media_src: Vec::new(),
            object_src: Vec::new(),
            script_src: Vec::new(),
        }
    }

    /// Fallback for other [Content-Security-Policy] directives
    ///
    /// [Content-Security-Policy]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Security-Policy
    pub fn add_default_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.default_src.push(directive.into());
        self
    }

    /// Specifies valid stylesheet sources
    pub fn add_style_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.style_src.push(directive.into());
        self
    }

    /// Specifies valid URLs which can be loaded via scripts
    pub fn add_connect_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.connect_src.push(directive.into());
        self
    }

    /// Specifies valid font sources, which are loaded using css [@font-face] attribute
    ///
    /// [@font-face]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
    pub fn add_font_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.font_src.push(directive.into());
        self
    }

    /// Specifies valid [<iframe>] and [<frame>] sources
    ///
    /// [<iframe>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe
    /// [<frame>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/frame
    pub fn add_frame_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.frame_src.push(directive.into());
        self
    }

    /// Specifies valid image and favicon sources
    pub fn add_image_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.img_src.push(directive.into());
        self
    }

    /// Specifies valid application manifest file sources
    pub fn add_manifest_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.manifest_src.push(directive.into());
        self
    }

    /// Specifies valid [<audio>], [<track>] and [<video>] sources
    ///
    /// [<audio>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio
    /// [<track>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track
    /// [<video>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video
    pub fn add_media_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.media_src.push(directive.into());
        self
    }

    /// Specifies valid [<object>], [<applet>] and [<embed>] sources
    ///
    /// [<object>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object
    /// [<applet>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/applet
    /// [<embed>]: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed
    pub fn add_object_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.object_src.push(directive.into());
        self
    }

    /// Specifies valid JavaScript sources
    pub fn add_script_src<S>( mut self, directive: S) -> Csp 
        where S: Into<Cow<'static, str>>
    {
        self.script_src.push(directive.into());
        self
    }
}

impl<'a> Into<Header<'static>> for &'a Csp {
    fn into(self) -> Header<'static> {
        let mut policy_string = String::with_capacity(200);

        if !self.default_src.is_empty() {
            policy_string.push_str(&format!("default-src {}; ", self.default_src.join(" ")));
        }

        if !self.style_src.is_empty() {
            policy_string.push_str(&format!("style-src {}; ", self.style_src.join(" ")));
        }

        if !self.connect_src.is_empty() {
            policy_string.push_str(&format!("connect-src {}; ", self.connect_src.join(" ")));
        }

        if !self.font_src.is_empty() {
            policy_string.push_str(&format!("font-src {}; ", self.font_src.join(" ")));
        }

        if !self.frame_src.is_empty() {
            policy_string.push_str(&format!("frame-src {}; ", self.frame_src.join(" ")));
        }

        if !self.img_src.is_empty() {
            policy_string.push_str(&format!("img-src {}; ", self.img_src.join(" ")));
        }

        if !self.manifest_src.is_empty() {
            policy_string.push_str(&format!("manifest-src {}; ", self.manifest_src.join(" ")));
        }

        if !self.media_src.is_empty() {
            policy_string.push_str(&format!("media-src {}; ", self.media_src.join(" ")));
        }

        if !self.object_src.is_empty() {
            policy_string.push_str(&format!("object-src {}; ", self.object_src.join(" ")));
        }

        if !self.script_src.is_empty() {
            policy_string.push_str(&format!("script-src {}; ", self.script_src.join(" ")));
        }

        Header::new(Csp::NAME, policy_string)
    }
}

/// Defaults to ---
impl Default for Csp {
    fn default() -> Csp {
        // FIXME: what default value?
        Csp::new()
    }
}
