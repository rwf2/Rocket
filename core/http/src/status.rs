use std::fmt;

/// Enumeration of HTTP status classes.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StatusClass {
    /// Indicates a provisional response: a status code of 1XX.
    Informational,
    /// Indicates that a request has succeeded: a status code of 2XX.
    Success,
    /// Indicates that further action needs to be taken by the user agent in
    /// order to fulfill the request: a status code of 3XX.
    Redirection,
    /// Intended for cases in which the client seems to have erred: a status
    /// code of 4XX.
    ClientError,
    /// Indicates cases in which the server is aware that it has erred or is
    /// incapable of performing the request: a status code of 5XX.
    ServerError,
    /// Indicates that the status code is nonstandard and unknown: all other
    /// status codes.
    Unknown
}

macro_rules! class_check_fn {
    ($func:ident, $type:expr, $variant:ident) => (
        /// Returns `true` if `self` is a `StatusClass` of
        #[doc=$type]
        /// Returns `false` otherwise.
        #[inline(always)]
        pub fn $func(&self) -> bool {
            *self == StatusClass::$variant
        }
    )
}

impl StatusClass {
    class_check_fn!(is_informational, "`Informational` (1XX).", Informational);
    class_check_fn!(is_success, "`Success` (2XX).", Success);
    class_check_fn!(is_redirection, "`Redirection` (3XX).", Redirection);
    class_check_fn!(is_client_error, "`ClientError` (4XX).", ClientError);
    class_check_fn!(is_server_error, "`ServerError` (5XX).", ServerError);
    class_check_fn!(is_unknown, "`Unknown`.", Unknown);
}

/// Structure representing an HTTP status: an integer code.
///
/// A `Status` should rarely be created directly. Instead, an associated
/// constant should be used; one is declared for every status defined in the
/// HTTP standard. If a custom status code _must_ be created, note that it is
/// not possible to set a custom reason phrase.
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Status;
///
/// // Create a status from a known constant.
/// let ok = Status::Ok;
/// assert_eq!(ok.code, 200);
/// assert_eq!(ok.reason(), Some("OK"));
///
/// let not_found = Status::NotFound;
/// assert_eq!(not_found.code, 404);
/// assert_eq!(not_found.reason(), Some("Not Found"));
///
/// // Or from a status code: `reason()` returns the phrase when known.
/// let gone = Status::new(410);
/// assert_eq!(gone.code, 410);
/// assert_eq!(gone.reason(), Some("Gone"));
///
/// // `reason()` returns `None` when unknown.
/// let custom = Status::new(599);
/// assert_eq!(custom.code, 599);
/// assert_eq!(custom.reason(), None);
/// ```
///
/// # Responding
///
/// To set a custom `Status` on a response, use a [`response::status`]
/// responder, which enforces correct status-based responses. Alternatively,
/// respond with `(Status, T)` where `T: Responder`, but beware that the
/// response may be invalid if it requires additional headers.
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::get;
/// use rocket::http::Status;
///
/// #[get("/")]
/// fn index() -> (Status, &'static str) {
///     (Status::NotFound, "Hey, there's no index!")
/// }
/// ```
///
/// [`response::status`]: ../response/status/index.html
#[derive(Debug, Clone, Copy)]
pub struct Status {
    /// The HTTP status code associated with this status.
    pub code: u16,
}

impl Default for Status {
    fn default() -> Self {
        Status::Ok
    }
}

macro_rules! ctrs {
    ($($code:expr, $code_str:expr, $name:ident => $reason:expr),+) => {
        $(
            #[doc="[`Status`] with code <b>"]
            #[doc=$code_str]
            #[doc="</b>."]
            #[allow(non_upper_case_globals)]
            pub const $name: Status = Status { code: $code };
        )+

        /// Creates a new `Status` with `code`. This should be used _only_ to
        /// construct non-standard HTTP statuses. Use an associated constant for
        /// standard statuses.
        ///
        /// # Example
        ///
        /// Create a custom `299` status:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// let custom = Status::new(299);
        /// assert_eq!(custom.code, 299);
        /// ```
        pub const fn new(code: u16) -> Status {
            Status { code }
        }

        /// Returns the class of a given status.
        ///
        /// # Example
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::{Status, StatusClass};
        ///
        /// let processing = Status::Processing;
        /// assert_eq!(processing.class(), StatusClass::Informational);
        ///
        /// let ok = Status::Ok;
        /// assert_eq!(ok.class(), StatusClass::Success);
        ///
        /// let see_other = Status::SeeOther;
        /// assert_eq!(see_other.class(), StatusClass::Redirection);
        ///
        /// let not_found = Status::NotFound;
        /// assert_eq!(not_found.class(), StatusClass::ClientError);
        ///
        /// let internal_error = Status::InternalServerError;
        /// assert_eq!(internal_error.class(), StatusClass::ServerError);
        ///
        /// let custom = Status::new(600);
        /// assert_eq!(custom.class(), StatusClass::Unknown);
        /// ```
        pub const fn class(self) -> StatusClass {
            match self.code / 100 {
                1 => StatusClass::Informational,
                2 => StatusClass::Success,
                3 => StatusClass::Redirection,
                4 => StatusClass::ClientError,
                5 => StatusClass::ServerError,
                _ => StatusClass::Unknown
            }
        }

        /// Returns a Status given a standard status code `code`. If `code` is
        /// not a known status code, `None` is returned.
        ///
        /// # Example
        ///
        /// Create a `Status` from a known `code`:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// let not_found = Status::from_code(404);
        /// assert_eq!(not_found, Some(Status::NotFound));
        /// ```
        ///
        /// Create a `Status` from an unknown `code`:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// let unknown = Status::from_code(600);
        /// assert!(unknown.is_none());
        /// ```
        pub const fn from_code(code: u16) -> Option<Status> {
            match code {
                $($code => Some(Status::$name),)+
                _ => None
            }
        }

        /// Returns the canonical reason phrase if `self` corresponds to a
        /// canonical, known status code. Otherwise, returns `None`.
        ///
        /// # Example
        ///
        /// Reason phrase from a known `code`:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// assert_eq!(Status::Created.reason(), Some("Created"));
        /// assert_eq!(Status::new(200).reason(), Some("OK"));
        /// ```
        ///
        /// Absent phrase from an unknown `code`:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// assert_eq!(Status::new(499).reason(), None);
        /// ```
        pub const fn reason(&self) -> Option<&'static str> {
            match self.code {
                $($code => Some($reason),)+
                _ => None
            }
        }

        /// Returns the canonical reason phrase if `self` corresponds to a
        /// canonical, known status code, or an unspecified but relevant reason
        /// phrase otherwise.
        ///
        /// # Example
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::Status;
        ///
        /// assert_eq!(Status::NotFound.reason_lossy(), "Not Found");
        /// assert_eq!(Status::new(100).reason_lossy(), "Continue");
        /// assert!(!Status::new(699).reason_lossy().is_empty());
        /// ```
        pub const fn reason_lossy(&self) -> &'static str {
            if let Some(lossless) = self.reason() {
                return lossless;
            }

            match self.class() {
                StatusClass::Informational => "Informational",
                StatusClass::Success => "Success",
                StatusClass::Redirection => "Redirection",
                StatusClass::ClientError => "Client Error",
                StatusClass::ServerError => "Server Error",
                StatusClass::Unknown => "Unknown"
            }
        }
    };
}

impl Status {
    ctrs! {
        100, "100", Continue => "Continue",
        101, "101", SwitchingProtocols => "Switching Protocols",
        102, "102", Processing => "Processing",
        200, "200", Ok => "OK",
        201, "201", Created => "Created",
        202, "202", Accepted => "Accepted",
        203, "203", NonAuthoritativeInformation => "Non-Authoritative Information",
        204, "204", NoContent => "No Content",
        205, "205", ResetContent => "Reset Content",
        206, "206", PartialContent => "Partial Content",
        207, "207", MultiStatus => "Multi-Status",
        208, "208", AlreadyReported => "Already Reported",
        226, "226", ImUsed => "IM Used",
        300, "300", MultipleChoices => "Multiple Choices",
        301, "301", MovedPermanently => "Moved Permanently",
        302, "302", Found => "Found",
        303, "303", SeeOther => "See Other",
        304, "304", NotModified => "Not Modified",
        305, "305", UseProxy => "Use Proxy",
        307, "307", TemporaryRedirect => "Temporary Redirect",
        308, "308", PermanentRedirect => "Permanent Redirect",
        400, "400", BadRequest => "Bad Request",
        401, "401", Unauthorized => "Unauthorized",
        402, "402", PaymentRequired => "Payment Required",
        403, "403", Forbidden => "Forbidden",
        404, "404", NotFound => "Not Found",
        405, "405", MethodNotAllowed => "Method Not Allowed",
        406, "406", NotAcceptable => "Not Acceptable",
        407, "407", ProxyAuthenticationRequired => "Proxy Authentication Required",
        408, "408", RequestTimeout => "Request Timeout",
        409, "409", Conflict => "Conflict",
        410, "410", Gone => "Gone",
        411, "411", LengthRequired => "Length Required",
        412, "412", PreconditionFailed => "Precondition Failed",
        413, "413", PayloadTooLarge => "Payload Too Large",
        414, "414", UriTooLong => "URI Too Long",
        415, "415", UnsupportedMediaType => "Unsupported Media Type",
        416, "416", RangeNotSatisfiable => "Range Not Satisfiable",
        417, "417", ExpectationFailed => "Expectation Failed",
        418, "418", ImATeapot => "I'm a teapot",
        421, "421", MisdirectedRequest => "Misdirected Request",
        422, "422", UnprocessableEntity => "Unprocessable Entity",
        423, "423", Locked => "Locked",
        424, "424", FailedDependency => "Failed Dependency",
        426, "426", UpgradeRequired => "Upgrade Required",
        428, "428", PreconditionRequired => "Precondition Required",
        429, "429", TooManyRequests => "Too Many Requests",
        431, "431", RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
        451, "451", UnavailableForLegalReasons => "Unavailable For Legal Reasons",
        500, "500", InternalServerError => "Internal Server Error",
        501, "501", NotImplemented => "Not Implemented",
        502, "502", BadGateway => "Bad Gateway",
        503, "503", ServiceUnavailable => "Service Unavailable",
        504, "504", GatewayTimeout => "Gateway Timeout",
        505, "505", HttpVersionNotSupported => "HTTP Version Not Supported",
        506, "506", VariantAlsoNegotiates => "Variant Also Negotiates",
        507, "507", InsufficientStorage => "Insufficient Storage",
        508, "508", LoopDetected => "Loop Detected",
        510, "510", NotExtended => "Not Extended",
        511, "511", NetworkAuthenticationRequired => "Network Authentication Required"
    }
}

impl fmt::Display for Status {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code, self.reason_lossy())
    }
}

impl std::hash::Hash for Status {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.code.hash(state)
    }
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        self.code.eq(&other.code)
    }
}

impl Eq for Status { }

impl PartialOrd for Status {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.code.partial_cmp(&other.code)
    }
}

impl Ord for Status {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.code.cmp(&other.code)
    }
}
