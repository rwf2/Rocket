//! Implementation of CORS based on [the fetch whatwg
//! spec](https://fetch.spec.whatwg.org/#http-cors-protocol).
//!
//! This fairing is appropriate when your whole site will follow the same CORS rules. It doesn't
//! yet support custom CORS on individual routes.
use rocket::{
    fairing,
    http::{ext::IntoOwned, uncased::Uncased, uri, ContentType, Method, Status},
    response::{Response, ResponseBuilder},
    Request,
};
use std::{borrow::Cow, collections::HashSet, error::Error as StdError, fmt, io, mem};

/// Generate compile-time constant header names.
macro_rules! hdrs {
    ($($s:expr),*) => {
        [$(Uncased { string: Cow::Borrowed($s) }),*]
    };
}

/// Headers that are allowed to be accessed in all CORS requests.
const ALLOWED_HEADERS: &'static [Uncased<'static>] =
    &hdrs!["Accept", "Accept-Language", "Content-Language"];

/// Headers that are allowed to be accessed in all responses to CORS requests.
const EXPOSED_HEADERS: &'static [Uncased<'static>] = &hdrs![
    "Cache-Control",
    "Content-Language",
    "Content-Type",
    "Expires",
    "Last-Modified",
    "Pragma"
];

/// the possibilities for allowed origin.
// This isn't public because the types may change after `http 1.0` is release, breaking backwards
// compat.
enum AllowedOrigin {
    /// A whitelist of origins that are allowed. (e.g. `http://my_host.tld:2123`,
    /// `https://example.com`).
    Some(HashSet<&'static str>),
    /// All origins are allowed (corresponds to "*")
    Any,
}

/// Adds Cross-origin resource sharing (CORS) support as a fairing.
pub struct CORS {
    /// The origins that will be accepted when responding to requests.
    allow_origin: AllowedOrigin,
    /// Which headers we allow the client to read in javascript.
    expose_headers: HashSet<Uncased<'static>>,
    /// Whether cookies should be sent.
    allow_credentials: bool,
    /// Which headers the client is allowed to send during the actual request (used in preflight)
    allow_headers: HashSet<Uncased<'static>>,
    /// All methods could be possibly allowed or not.
    allow_methods: HashSet<Method>,
    /// The maximum time between a preflight request and the real request.
    max_age: Option<usize>,
}

impl CORS {
    // Config/setup methods
    // ====================

    /// Helper to create empty CORS object.
    fn new(allow_origin: AllowedOrigin) -> CORS {
        CORS {
            allow_origin,
            expose_headers: HashSet::new(),
            allow_credentials: false,
            allow_headers: HashSet::new(),
            allow_methods: HashSet::new(),
            max_age: None,
        }
    }

    /// Create a CORS fairing from a comma-separated list of origins, or `*` to allow for all
    /// origins.
    ///
    /// The CORS spec states that if the origin matches the allowed origin, we set that as the
    /// `Access-Control-Allow-Origin` header, or set it to `*` if we support any origin. We extend
    /// this slightly by allowing multiple origins, and if a request origin is in the list, we
    /// reflect it back on its own, thereby complying with the spec.
    pub fn from_origin(origin: &'static str) -> Result<CORS, OriginError> {
        let allow_origin = match origin.trim() {
            "*" => AllowedOrigin::Any,
            o => {
                AllowedOrigin::Some(
                    o.split(',')
                        .map(|o| o.trim())
                        .filter(|o| !o.is_empty())
                        .map(|o| {
                            // we parse the origin as a url to check it is valid.
                            let parsed = uri::Absolute::parse(o)
                                .map_err(|e| OriginError::from_parts(origin, e))?;
                            match parsed.scheme() {
                                "http" | "https" => (),
                                other => {
                                    return Err(OriginError::from_parts(
                                        origin,
                                        OriginErrorKind::SchemeNotHyperText(other.to_owned()),
                                    ));
                                }
                            };
                            if let None = parsed.authority() {
                                return Err(OriginError::from_parts(
                                    origin,
                                    OriginErrorKind::HasNoAuthority,
                                ));
                            };
                            if let Some(uri_origin) = parsed.origin() {
                                return Err(OriginError::from_parts(
                                    origin,
                                    OriginErrorKind::HasOrigin(uri_origin.to_owned()),
                                ));
                            };
                            Ok(o)
                        })
                        .collect::<Result<HashSet<&'static str>, OriginError>>()?,
                )
            }
        };

        Ok(CORS::new(allow_origin))
    }

    /// Allow all origins (`*` in the header).
    pub fn any() -> CORS {
        CORS::new(AllowedOrigin::Any)
    }

    /// Whether credentials are allowed to be present in cross-origin requests.
    pub fn allow_credentials(mut self, allow_credentials: bool) -> CORS {
        self.allow_credentials = allow_credentials;
        self
    }

    /// The http methods allowed in cross-origin requests.
    pub fn allow_methods(mut self, methods: impl IntoIterator<Item = Method>) -> CORS {
        for method in methods.into_iter() {
            self.allow_methods.insert(method);
        }
        self
    }

    /// These are used for preflight request (OPTIONS) to specify which headers are allowed in the
    /// real request.
    ///
    /// See [the spec](https://fetch.spec.whatwg.org/#http-cors-protocol) for a list of headers
    /// that are allowed by default.
    pub fn allow_headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> CORS {
        for header in headers.into_iter() {
            let header = Uncased::new(header);
            if ALLOWED_HEADERS.contains(&header) {
                warn!(
                    "Header \"{}\" is allowed by default and does not need to be included",
                    header
                );
            } else {
                self.allow_headers.insert(header);
            }
        }
        self
    }

    /// Which headers in the response should be exposed to the client javascript.
    pub fn exposed_headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> CORS {
        for header in headers.into_iter() {
            let header = Uncased::new(header);
            if EXPOSED_HEADERS.contains(&header) {
                warn!(
                    "Header \"{}\" is allowed by default and does not need to be included",
                    header
                );
            } else {
                self.expose_headers.insert(header);
            }
        }
        self
    }

    /// The maximum amount of time that a preflight request should be valid for. After this, the
    /// client should repeat the preflight before the main request. In seconds.
    pub fn max_age(mut self, max_age: usize) -> CORS {
        self.max_age = Some(max_age);
        self
    }

    // Request handling methods
    // ========================

    /// Handle a preflight CORS request (method OPTIONS)
    fn handle_preflight(&self, request: &Request, response: &mut Response) {
        // Only handle requests that weren't handled explicitally.
        if response.status() != Status::NotFound {
            return;
        }

        // swap out the old response and drop it
        let mut cors_response = Response::build().status(Status::Ok).finalize();
        mem::swap(&mut cors_response, response);
        drop(cors_response);

        // Run check and set the CORS headers.
        if !self.check_origin(request, response) {
            return;
        }
        if !self.check_method(request, response) {
            return;
        }
        if !self.check_headers(request, response) {
            return;
        }
        self.add_allow_credentials(response);
        self.add_allow_headers(response);
    }

    /// Modify a standard request to add CORS.
    fn handle_cors(&self, request: &Request, response: &mut Response) {
        if !self.check_origin(request, response) {
            return;
        }
        self.add_allow_credentials(response);
        self.add_allow_headers(response);
    }

    /// If the origin check passes, add the related header, else replace with an error response.
    fn check_origin(&self, request: &Request, response: &mut Response) -> bool {
        match self.allow_origin {
            AllowedOrigin::Some(ref origins) => match request.headers().get_one("Origin") {
                Some(origin) => {
                    if let Some(cors_origin) = origins.get(origin) {
                        set_origin_header(response, Some(cors_origin));
                        true
                    } else {
                        unauthorized(response, "origin not valid");
                        false
                    }
                }
                _ => {
                    unauthorized(response, "origin not present");
                    false
                }
            },
            AllowedOrigin::Any => {
                set_origin_header(response, None);
                true
            }
        }
    }

    /// If the method check passes, add the related header, else replace with an error response.
    fn check_method(&self, request: &Request, response: &mut Response) -> bool {
        if let Some(method) = request.headers().get_one("Access-Control-Request-Method") {
            match <Method as std::str::FromStr>::from_str(&method) {
                Ok(method) if self.allow_methods.contains(&method) => {
                    set_methods_header(response, &self.allow_methods);
                    true
                }
                _ => {
                    unauthorized(response, "requested method not valid");
                    false
                }
            }
        } else {
            true
        }
    }

    /// If the allowed headers check passes, add the related header, else replace with an error response.
    fn check_headers(&self, request: &Request, response: &mut Response) -> bool {
        if let Some(allow_headers) = request.headers().get_one("Access-Control-Request-Headers") {
            for header in allow_headers.split(',').map(|s| Uncased::new(s.trim())) {
                if !(self.allow_headers.contains(&header) || ALLOWED_HEADERS.contains(&header)) {
                    unauthorized(response, "a requested header is not supported");
                    return false;
                }
            }
        }
        if self.allow_headers.len() > 0 {
            set_headers_header(response, &self.allow_headers);
        }
        true
    }

    fn add_allow_credentials(&self, response: &mut Response) {
        if self.allow_credentials {
            response.set_raw_header("Access-Control-Allow-Credentials", "true");
        }
    }

    fn add_allow_headers(&self, response: &mut Response) {
        if self.expose_headers.len() > 0 {
            set_exposed_headers_header(response, &self.expose_headers);
        }
    }
}

impl fairing::Fairing for CORS {
    fn info(&self) -> fairing::Info {
        use rocket::fairing::{Info, Kind};
        Info {
            name: "Cross-origin resource sharing (CORS) support",
            kind: Kind::Response,
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if request.method() == Method::Options {
            self.handle_preflight(request, response)
        } else {
            self.handle_cors(request, response)
        }
    }
}

// Error handling for CORS
// -----------------------

/// Ways we can fail to parse a CORS origin.
#[derive(Debug)]
pub enum OriginErrorKind {
    ParsingFailed(uri::Error<'static>),
    SchemeNotHyperText(String),
    HasNoAuthority,
    HasOrigin(uri::Origin<'static>),
}

impl<'a> From<uri::Error<'a>> for OriginErrorKind {
    fn from(e: uri::Error) -> Self {
        OriginErrorKind::ParsingFailed(e.into_owned())
    }
}

/// A failure in parsing a CORS origin.
#[derive(Debug)]
pub struct OriginError {
    uri: String,
    kind: OriginErrorKind,
}

impl OriginError {
    fn from_parts(uri: impl Into<String>, kind: impl Into<OriginErrorKind>) -> Self {
        OriginError {
            uri: uri.into(),
            kind: kind.into(),
        }
    }
}

impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::OriginErrorKind::*;
        write!(f, r#"error in uri "{}": "#, self.uri)?;
        match &self.kind {
            ParsingFailed(inner) => fmt::Display::fmt(inner, f),
            SchemeNotHyperText(scheme) => write!(
                f,
                r#"exepected scheme to be "http" or "https", found "{}""#,
                scheme
            ),

            HasNoAuthority => f.write_str("the uri should have an authority, none found"),
            HasOrigin(origin) => write!(f, r#"expected empty origin, found "{}"#, origin),
        }
    }
}

impl StdError for OriginError {}

// Helper methdos
// --------------

/// Replace the current response with one representing unauthorized.
///
/// It is important that these methods don't leak any information they shouldn't.
fn unauthorized(response: &mut Response, msg: &'static str) {
    let mut cors_response = Response::build()
        .status(Status::Unauthorized)
        .header(ContentType::Plain)
        .sized_body(io::Cursor::new(msg))
        .finalize();
    mem::swap(response, &mut cors_response);
    // drop original response.
}

/// Set the origin header of the response to the given origin, or "*" if None.
///
/// It would be nice to remove the `'static` restriction on the origin, to allow them to be set
/// dynamically, but I'm not sure the typechecker (or me) can check our memory safety rules.
///
/// Altenratively, they could be interned so there is at most one allocation for each origin, but
/// the current way requires no allocations.
fn set_origin_header(res: &mut Response, origin: Option<&&'static str>) {
    res.set_raw_header(
        "Access-Control-Allow-Origin",
        origin.map(|o| *o).unwrap_or("*"),
    );
}

/// Set the methods header of the response to the given allowed methods.
fn set_methods_header(res: &mut Response, methods: &HashSet<Method>) {
    // for now we build the string for every request, but we could do some
    // interning. It's not automatic this would be faster, but could bench to
    // see.
    //
    // longest method is 7 bytes.
    let mut methods_str = String::with_capacity(methods.len() * 7);
    for method in methods {
        methods_str.push_str(method.as_str());
        methods_str.push_str(", ");
    }
    methods_str.pop(); // remove last ' '
    methods_str.pop(); // remove last ','
    res.set_raw_header("Access-Control-Allow-Methods", methods_str);
}

/// Set the allowed headers header of the response to the given allowed headers.
fn set_headers_header(res: &mut Response, headers: &HashSet<Uncased<'static>>) {
    // I guess that most headers are less than 16 bytes
    let mut headers_str = String::with_capacity(headers.len() * 18);
    for header in headers {
        headers_str.push_str(header.as_str());
        headers_str.push_str(", ");
    }
    headers_str.pop(); // remove last ' '
    headers_str.pop(); // remove last ','
    res.set_raw_header("Access-Control-Allow-Headers", headers_str);
}

/// Set the allowed headers header of the response to the given allowed headers.
fn set_exposed_headers_header(res: &mut Response, headers: &HashSet<Uncased<'static>>) {
    // I guess that most header names are less than 16 bytes
    let headers_str = concat_strs(headers.iter(), ", ", headers.len(), 16);
    res.set_raw_header("Access-Control-Expose-Headers", headers_str);
}

fn concat_strs(
    strs: impl Iterator<Item = impl AsRef<str>>,
    join: &'static str,
    len: usize,
    guess_size: usize,
) -> String {
    let mut output = String::with_capacity(len * (guess_size + join.len()));
    for (idx, s) in strs.enumerate() {
        output.push_str(s.as_ref());
        if idx < len - 1 {
            output.push_str(", ");
        }
    }
    output
}
