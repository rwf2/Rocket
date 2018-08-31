use rocket::http::uri::Uri;
use rocket::http::Header;

///[Referrer-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy) description.
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl Default for ReferrerPolicy {
    fn default() -> ReferrerPolicy {
        ReferrerPolicy::NoReferrer
    }
}

impl<'a, 'b> From<&'a ReferrerPolicy> for Header<'b> {
    fn from(policy: &ReferrerPolicy) -> Header<'b> {
        let policy_string = match policy {
            ReferrerPolicy::NoReferrer => String::from("no-referrer"),
            ReferrerPolicy::NoReferrerWhenDowngrade => String::from("no-referrer-when-downgrade"),
            ReferrerPolicy::Origin => String::from("origin"),
            ReferrerPolicy::OriginWhenCrossOrigin => String::from("origin-when-cross-origin"),
            ReferrerPolicy::SameOrigin => String::from("same-origin"),
            ReferrerPolicy::StrictOrigin => String::from("strict-origin"),
            ReferrerPolicy::StrictOriginWhenCrossOrigin => {
                String::from("strict-origin-when-cross-origin")
            }
            ReferrerPolicy::UnsafeUrl => String::from("unsafe-url"),
        };
        Header::new("Referrer-Policy", policy_string)
    }
}

pub type MaxAge = u32;

///30 days in seconds, see the Expect-CT IETF
///   [draft](https://tools.ietf.org/html/draft-ietf-httpbis-expect-ct-03#page-15) for more.
pub const EXPECTCT_MAX_AGE_DEFAULT: u32 = 2592000;

///[Expect-CT](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Expect-CT) description.
pub enum ExpectCTPolicy<'a> {
    Enforce(MaxAge),
    Report(MaxAge, Uri<'a>),
    ReportAndEnforce(MaxAge, Uri<'a>),
}

impl<'a> Default for ExpectCTPolicy<'a> {
    fn default() -> ExpectCTPolicy<'a> {
        ExpectCTPolicy::Enforce(EXPECTCT_MAX_AGE_DEFAULT)
    }
}

impl<'a, 'b> From<&'a ExpectCTPolicy<'a>> for Header<'b> {
    fn from(policy: &ExpectCTPolicy<'a>) -> Header<'b> {
        let policy_string = match policy {
            ExpectCTPolicy::Enforce(max_age) => format!("max-age={}, enforce", max_age),
            ExpectCTPolicy::Report(max_age, url) => {
                format!("max-age={}, report-uri=\"{}\"", max_age, url)
            }
            ExpectCTPolicy::ReportAndEnforce(max_age, url) => {
                format!("max-age={}, enforce, report-uri=\"{}\"", max_age, url)
            }
        };
        Header::new("Expect-CT", policy_string)
    }
}

///[X-Content-Type-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options) description.
pub enum NoSniffPolicy {
    Enable,
}

impl Default for NoSniffPolicy {
    fn default() -> NoSniffPolicy {
        NoSniffPolicy::Enable
    }
}

impl<'a, 'b> From<&'a NoSniffPolicy> for Header<'b> {
    fn from(_policy: &NoSniffPolicy) -> Header<'b> {
        Header::new("X-Content-Type-Options", "nosniff")
    }
}

///[Strict-Transport-Security](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security) description.
pub enum HSTSPolicy {
    Enable(MaxAge),
    IncludeSubDomains(MaxAge),
    Preload(MaxAge),
}

//XXX add debug setting.
///One year in seconds, a setting for production environments.
pub const HSTS_MAX_AGE_DEFAULT: u32 = 31536000;

impl Default for HSTSPolicy {
    fn default() -> HSTSPolicy {
        HSTSPolicy::Enable(HSTS_MAX_AGE_DEFAULT)
    }
}

impl<'a, 'b> From<&'a HSTSPolicy> for Header<'b> {
    fn from(policy: &HSTSPolicy) -> Header<'b> {
        let policy_string = match policy {
            HSTSPolicy::Enable(max_age) => format!("max-age={}", max_age),
            HSTSPolicy::IncludeSubDomains(max_age) => {
                format!("max-age={} ; includeSubDomains", max_age)
            }
            HSTSPolicy::Preload(max_age) => format!("max-age={} ; preload", max_age),
        };
        Header::new("Strict-Transport-Security", policy_string)
    }
}

///[X-Frame-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options) description.
pub enum FramePolicy<'a> {
    SameOrigin,
    Deny,
    AllowFrom(Uri<'a>),
}

impl<'a> Default for FramePolicy<'a> {
    fn default() -> FramePolicy<'a> {
        FramePolicy::SameOrigin
    }
}

impl<'a, 'b> From<&'a FramePolicy<'a>> for Header<'b> {
    fn from(policy: &FramePolicy<'a>) -> Header<'b> {
        let policy_string = match policy {
            FramePolicy::Deny => String::from("DENY"),
            FramePolicy::SameOrigin => String::from("SAMEORIGIN"),
            FramePolicy::AllowFrom(uri) => format!("ALLOW-FROM {}", uri),
        };
        Header::new("X-Frame-Options", policy_string)
    }
}

///[X-XSS-Protection](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection) description.
pub enum XSSPolicy<'a> {
    Disable,
    Enable,
    EnableBlock,
    EnableReport(Uri<'a>),
}

impl<'a> Default for XSSPolicy<'a> {
    fn default() -> XSSPolicy<'a> {
        XSSPolicy::EnableBlock
    }
}

impl<'a, 'b> From<&'a XSSPolicy<'a>> for Header<'b> {
    fn from(policy: &XSSPolicy) -> Header<'b> {
        let policy_string = match policy {
            XSSPolicy::Disable => String::from("0"),
            XSSPolicy::Enable => String::from("1"),
            XSSPolicy::EnableBlock => String::from("1; mode=block"),
            XSSPolicy::EnableReport(u) => format!("{}{}", "1; report=", u),
        };
        Header::new("X-XSS-Protection", policy_string)
    }
}
