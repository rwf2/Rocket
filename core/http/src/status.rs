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

/// Structure representing an HTTP status: an integer code and a reason phrase.
///
/// # Usage
///
/// Status classes should rarely be created directly. Instead, an associated
/// constant should be used; one is declared for every status defined
/// in the HTTP standard.
///
/// ## Example
///
/// A status of `200 OK` can be instantiated via the `Ok` constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Status;
///
/// # #[allow(unused_variables)]
/// let ok = Status::Ok;
/// ```
///
/// A status of `404 Not Found` can be instantiated via the `NotFound` constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Status;
///
/// # #[allow(unused_variables)]
/// let not_found = Status::NotFound;
/// ```
///
/// The code and phrase can be retrieved directly:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Status;
///
/// let not_found = Status::NotFound;
///
/// assert_eq!(not_found.code, 404);
/// assert_eq!(not_found.reason, "Not Found");
/// assert_eq!(not_found.to_string(), "404 Not Found".to_string());
/// ```
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Status {
    /// The HTTP status code associated with this status.
    pub code: u16,
    /// The HTTP reason phrase associated with this status.
    pub reason: &'static str
}

macro_rules! ctrs {
    ($($code:expr, $code_str:expr, $name:ident => $reason:expr),+) => {
        $(
            #[doc="[`Status`] with code <b>"]
            #[doc=$code_str]
            #[doc="</b> and reason <i>"]
            #[doc=$reason]
            #[doc="</i>."]
            #[allow(non_upper_case_globals)]
            pub const $name: Status = Status { code: $code, reason: $reason };
         )+

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
        /// let not_found = Status::from_code(600);
        /// assert!(not_found.is_none());
        /// ```
        pub fn from_code(code: u16) -> Option<Status> {
            match code {
                $($code => Some(Status::$name),)+
                _ => None
            }
        }
    };
}

impl Status {
    /// Creates a new `Status` with `code` and `reason`. This should be used _only_
    /// to construct non-standard HTTP statuses. Use an associated constant for
    /// standard statuses.
    ///
    /// # Example
    ///
    /// Create a custom `299 Somewhat Successful` status:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Status;
    ///
    /// let custom = Status::new(299, "Somewhat Successful");
    /// assert_eq!(custom.to_string(), "299 Somewhat Successful".to_string());
    /// ```
    #[inline(always)]
    pub fn new(code: u16, reason: &'static str) -> Status {
        Status { code, reason }
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
    /// let custom = Status::new(600, "Bizarre");
    /// assert_eq!(custom.class(), StatusClass::Unknown);
    /// ```
    pub fn class(&self) -> StatusClass {
        match self.code / 100 {
            1 => StatusClass::Informational,
            2 => StatusClass::Success,
            3 => StatusClass::Redirection,
            4 => StatusClass::ClientError,
            5 => StatusClass::ServerError,
            _ => StatusClass::Unknown
        }
    }

    /// Returns a status from a given status code. If the status code is a
    /// standard code, then the reason phrase is populated accordingly.
    /// Otherwise the reason phrase is set to "<unknown code>".
    #[inline]
    #[doc(hidden)]
    pub fn raw(code: u16) -> Status {
        match Status::from_code(code) {
            Some(status) => status,
            None => Status::new(code, "<unknown code>")
        }
    }

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
        write!(f, "{} {}", self.code, self.reason)
    }
}
