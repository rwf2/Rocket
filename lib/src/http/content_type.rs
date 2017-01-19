use std::borrow::{Borrow, Cow};
use std::str::FromStr;
use std::fmt;

use http::Header;
use http::hyper::mime::Mime;
use http::ascii::{uncased_eq, UncasedAscii};
use router::Collider;

/// Representation of HTTP Content-Types.
///
/// # Usage
///
/// ContentTypes should rarely be created directly. Instead, an associated
/// constant should be used; one is declared for most commonly used content
/// types.
///
/// ## Example
///
/// A Content-Type of `text/html; charset=utf-8` can be insantiated via the
/// `HTML` constant:
///
/// ```rust
/// use rocket::http::ContentType;
///
/// let html = ContentType::HTML;
/// ```
///
/// # Header
///
/// `ContentType` implements `Into<Header>`. As such, it can be used in any
/// context where an `Into<Header>` is expected:
///
/// ```rust
/// use rocket::http::ContentType;
/// use rocket::response::Response;
///
/// let response = Response::build().header(ContentType::HTML).finalize();
/// ```
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ContentType {
    /// The "type" component of the Content-Type.
    pub ttype: UncasedAscii<'static>,
    /// The "subtype" component of the Content-Type.
    pub subtype: UncasedAscii<'static>,
    /// Semicolon-seperated parameters associated with the Content-Type.
    pub params: Option<UncasedAscii<'static>>
}

macro_rules! ctr_params {
    () => (None);
    ($param:expr) => (Some(UncasedAscii { string: Cow::Borrowed($param) }));
}

macro_rules! ctrs {
    ($($str:expr, $name:ident, $check_name:ident =>
            $top:expr, $sub:expr $(; $param:expr),*),+) => {
        $(
            #[doc="[ContentType](struct.ContentType.html) for <b>"]
            #[doc=$str]
            #[doc="</b>: <i>"]
            #[doc=$top]
            #[doc="/"]
            #[doc=$sub]
            $(#[doc="; "] #[doc=$param])*
            #[doc="</i>"]
            #[allow(non_upper_case_globals)]
            pub const $name: ContentType = ContentType {
                ttype: UncasedAscii { string: Cow::Borrowed($top) },
                subtype: UncasedAscii { string: Cow::Borrowed($sub) },
                params: ctr_params!($($param)*)
            };
         )+

            /// Returns `true` if this ContentType is known to Rocket, that is,
            /// there is an associated constant for `self`.
            pub fn is_known(&self) -> bool {
                $(if self.$check_name() { return true })+
                false
            }

        $(
            #[doc="Returns `true` if `self` is a <b>"]
            #[doc=$str]
            #[doc="</b> ContentType: <i>"]
            #[doc=$top]
            #[doc="</i>/<i>"]
            #[doc=$sub]
            #[doc="</i>."]
            ///
            /// Paramaters are not taken into account when doing this check.
            #[inline(always)]
            pub fn $check_name(&self) -> bool {
                self.ttype == $top && self.subtype == $sub
            }
         )+
    };
}

impl ContentType {
    ctrs! {
        "any", Any, is_any => "*", "*",
        "HTML", HTML, is_html => "text", "html" ; "charset=utf-8",
        "Plain", Plain, is_plain => "text", "plain" ; "charset=utf-8",
        "JSON", JSON, is_json => "application", "json",
        "form", Form, is_form => "application", "x-www-form-urlencoded",
        "JavaScript", JavaScript, is_javascript => "application", "javascript",
        "CSS", CSS, is_css => "text", "css" ; "charset=utf-8",
        "data form", DataForm, is_data_form => "multipart", "form-data",
        "XML", XML, is_xml => "text", "xml" ; "charset=utf-8",
        "CSV", CSV, is_csv => "text", "csv" ; "charset=utf-8",
        "PNG", PNG, is_png => "image", "png",
        "GIF", GIF, is_gif => "image", "gif",
        "BMP", BMP, is_bmp => "image", "bmp",
        "JPEG", JPEG, is_jpeg => "image", "jpeg",
        "SVG", SVG, is_svg => "image", "svg+xml",
        "PDF", PDF, is_pdf => "application", "pdf"
    }

    /// Returns the Content-Type associated with the extension `ext`. Not all
    /// extensions are recognized. If an extensions is not recognized, then this
    /// method returns a ContentType of `Any`. The currently recognized
    /// extensions are: txt, html, htm, xml, js, css, json, png, gif, bmp, jpeg,
    /// jpg, and pdf.
    ///
    /// # Example
    ///
    /// A recognized content type:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let xml = ContentType::from_extension("xml");
    /// assert!(xml.is_xml());
    /// ```
    ///
    /// An unrecognized content type:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let foo = ContentType::from_extension("foo");
    /// assert!(foo.is_any());
    /// ```
    pub fn from_extension(ext: &str) -> ContentType {
        match ext {
            x if uncased_eq(x, "txt") => ContentType::Plain,
            x if uncased_eq(x, "html") => ContentType::HTML,
            x if uncased_eq(x, "htm") => ContentType::HTML,
            x if uncased_eq(x, "xml") => ContentType::XML,
            x if uncased_eq(x, "csv") => ContentType::CSV,
            x if uncased_eq(x, "js") => ContentType::JavaScript,
            x if uncased_eq(x, "css") => ContentType::CSS,
            x if uncased_eq(x, "json") => ContentType::JSON,
            x if uncased_eq(x, "png") => ContentType::PNG,
            x if uncased_eq(x, "gif") => ContentType::GIF,
            x if uncased_eq(x, "bmp") => ContentType::BMP,
            x if uncased_eq(x, "jpeg") => ContentType::JPEG,
            x if uncased_eq(x, "jpg") => ContentType::JPEG,
            x if uncased_eq(x, "svg") => ContentType::SVG,
            x if uncased_eq(x, "pdf") => ContentType::PDF,
            _ => ContentType::Any
        }
    }

    /// Creates a new `ContentType` with type `ttype` and subtype `subtype`.
    /// This should _only_ be used to construct uncommon Content-Types or custom
    /// Content-Types. Use an associated constant for common Content-Types.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-person` Content-Type:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::new("application", "x-person");
    /// assert_eq!(custom.to_string(), "application/x-person".to_string());
    /// ```
    #[inline(always)]
    pub fn new<T, S>(ttype: T, subtype: S) -> ContentType
        where T: Into<Cow<'static, str>>, S: Into<Cow<'static, str>>
    {
        ContentType::with_params::<T, S, T>(ttype, subtype, None)
    }

    /// Creates a new `ContentType` with type `ttype`, subtype `subtype`, and
    /// optionally parameters `params`, a semicolon-seperated list of
    /// parameters. This should be _only_ to construct uncommon Content-Types or
    /// custom Content-Types. Use an associated constant for common
    /// Content-Types.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-id; id=1` Content-Type:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let id = ContentType::with_params("application", "x-id", Some("id=1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    #[inline(always)]
    pub fn with_params<T, S, P>(ttype: T, subtype: S, params: Option<P>) -> ContentType
        where T: Into<Cow<'static, str>>,
              S: Into<Cow<'static, str>>,
              P: Into<Cow<'static, str>>
    {
        ContentType {
            ttype: UncasedAscii::from(ttype),
            subtype: UncasedAscii::from(subtype),
            params: params.map(|p| UncasedAscii::from(p))
        }
    }

    /// Returns an iterator over the (key, value) pairs of the Content-Type's
    /// parameter list. The iterator will be empty if the Content-Type has no
    /// parameters.
    ///
    /// # Example
    ///
    /// The `ContentType::Plain` type has one parameter: `charset=utf-8`:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let plain = ContentType::Plain;
    /// let plain_params: Vec<_> = plain.params().collect();
    /// assert_eq!(plain_params, vec![("charset", "utf-8")]);
    /// ```
    ///
    /// The `ContentType::PNG` type has no parameters:
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let png = ContentType::PNG;
    /// assert_eq!(png.params().count(), 0);
    /// ```
    #[inline(always)]
    pub fn params<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a str)> + 'a {
        let params = match self.params {
            Some(ref params) => params.as_str(),
            None => ""
        };

        params.split(";")
            .filter_map(|param| {
                let mut kv = param.split("=");
                match (kv.next(), kv.next()) {
                    (Some(key), Some(val)) => Some((key.trim(), val.trim())),
                    _ => None
                }
            })
    }
}

impl Default for ContentType {
    /// Returns a ContentType of `Any`, or `*/*`.
    #[inline(always)]
    fn default() -> ContentType {
        ContentType::Any
    }
}

#[doc(hidden)]
impl<T: Borrow<Mime>> From<T> for ContentType {
    default fn from(mime: T) -> ContentType {
        let mime: Mime = mime.borrow().clone();
        ContentType::from(mime)
    }
}

#[doc(hidden)]
impl From<Mime> for ContentType {
    fn from(mime: Mime) -> ContentType {
        let params = match mime.2.len() {
            0 => None,
            _ => {
                Some(mime.2.into_iter()
                    .map(|(attr, value)| format!("{}={}", attr, value))
                    .collect::<Vec<_>>()
                    .join("; "))
            }
        };

        ContentType::with_params(mime.0.to_string(), mime.1.to_string(), params)
    }
}

fn is_valid_char(c: char) -> bool {
    match c {
        '0'...'9' | 'A'...'Z' | '^'...'~' | '#'...'\''
            | '!' | '*' | '+' | '-' | '.'  => true,
        _ => false
    }
}

fn is_valid_token(string: &str) -> bool {
    string.len() >= 1 && string.chars().all(is_valid_char)
}

impl FromStr for ContentType {
    type Err = &'static str;

    /// Parses a ContentType from a given Content-Type header value.
    ///
    /// # Examples
    ///
    /// Parsing an `application/json`:
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let json = ContentType::from_str("application/json").unwrap();
    /// assert!(json.is_known());
    /// assert_eq!(json, ContentType::JSON);
    /// ```
    ///
    /// Parsing a content-type extension:
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::from_str("application/x-custom").unwrap();
    /// assert!(!custom.is_known());
    /// assert_eq!(custom.ttype, "application");
    /// assert_eq!(custom.subtype, "x-custom");
    /// ```
    ///
    /// Parsing an invalid Content-Type value:
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::from_str("application//x-custom");
    /// assert!(custom.is_err());
    /// ```
    fn from_str(raw: &str) -> Result<ContentType, &'static str> {
        let slash = match raw.find('/') {
            Some(i) => i,
            None => return Err("Missing / in MIME type."),
        };

        let top_s = &raw[..slash];
        let (sub_s, params) = match raw.find(';') {
            Some(j) => (raw[(slash + 1)..j].trim_right(), Some(raw[(j + 1)..].trim_left())),
            None => (&raw[(slash + 1)..], None),
        };

        if top_s.len() < 1 || sub_s.len() < 1 {
            return Err("Empty string.");
        }

        if !is_valid_token(top_s) || !is_valid_token(sub_s) {
            return Err("Invalid characters in type or subtype.");
        }

        let mut trimmed_params = vec![];
        for param in params.into_iter().flat_map(|p| p.split(';')) {
            let param = param.trim_left();
            for (i, split) in param.split('=').enumerate() {
                if split.trim() != split {
                    return Err("Whitespace not allowed around = character.");
                }

                match i {
                    0 => if !is_valid_token(split) {
                        return Err("Invalid parameter name.");
                    },
                    1 => if !((split.starts_with('"') && split.ends_with('"'))
                                || is_valid_token(split)) {
                        return Err("Invalid parameter value.");
                    },
                    _ => return Err("Malformed parameter.")
                }
            }

            trimmed_params.push(param);
        }

        let (ttype, subtype) = (top_s.to_string(), sub_s.to_string());
        let params = params.map(|_| trimmed_params.join(";"));
        Ok(ContentType::with_params(ttype, subtype, params))
    }
}

impl fmt::Display for ContentType {
    /// Formats the ContentType as an HTTP Content-Type value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// let ct = format!("{}", ContentType::JSON);
    /// assert_eq!(ct, "application/json");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.ttype, self.subtype)?;

        if let Some(ref params) = self.params {
            write!(f, "; {}", params)?;
        }

        Ok(())
    }
}

/// Creates a new `Header` with name `Content-Type` and the value set to the
/// HTTP rendering of this Content-Type.
impl Into<Header<'static>> for ContentType {
    #[inline]
    fn into(self) -> Header<'static> {
        Header::new("Content-Type", self.to_string())
    }
}

impl Collider for ContentType {
    fn collides_with(&self, other: &ContentType) -> bool {
        let collide = |a, b| a == "*" || b == "*" || a == b;
        collide(&self.ttype, &other.ttype) && collide(&self.subtype, &other.subtype)
    }
}

#[cfg(test)]
mod test {
    use super::ContentType;
    use std::str::FromStr;

    macro_rules! assert_no_parse {
        ($string:expr) => ({
            let result = ContentType::from_str($string);
            if !result.is_err() {
                println!("{} parsed unexpectedly!", $string);
            }

            assert!(result.is_err());
        });
    }

    macro_rules! assert_parse {
        ($string:expr) => ({
            let result = ContentType::from_str($string);
            if let Err(e) = result {
                println!("{:?} failed to parse: {}", $string, e);
            }

            result.unwrap()
        });

        ($string:expr, $ct:expr) => ({
            let c = assert_parse!($string);
            assert_eq!(c.ttype, $ct.ttype);
            assert_eq!(c.subtype, $ct.subtype);
            assert_eq!(c.params, $ct.params);
            c
        })
    }

    #[test]
    fn test_simple() {
        assert_parse!("application/json", ContentType::JSON);
        assert_parse!("*/json", ContentType::new("*", "json"));
        assert_parse!("text/html;charset=utf-8", ContentType::HTML);
        assert_parse!("text/html ; charset=utf-8", ContentType::HTML);
        assert_parse!("text/html ;charset=utf-8", ContentType::HTML);
        assert_parse!("TEXT/html;charset=utf-8", ContentType::HTML);
        assert_parse!("*/*", ContentType::Any);
        assert_parse!("application/*", ContentType::new("application", "*"));
        assert_parse!("image/svg+xml", ContentType::SVG);
    }

    #[test]
    fn test_params() {
        assert_parse!("*/*;a=1;b=2;c=3;d=4",
            ContentType::with_params("*", "*", Some("a=1;b=2;c=3;d=4")));
        assert_parse!("*/*; a=1;   b=2; c=3;d=4",
            ContentType::with_params("*", "*", Some("a=1;b=2;c=3;d=4")));
        assert_parse!("application/*;else=1",
            ContentType::with_params("application", "*", Some("else=1")));
        assert_parse!("*/*;charset=utf-8;else=1",
            ContentType::with_params("*", "*", Some("charset=utf-8;else=1")));
        assert_parse!("*/*;    charset=utf-8;   else=1",
            ContentType::with_params("*", "*", Some("charset=utf-8;else=1")));
        assert_parse!("*/*;    charset=\"utf-8\";   else=1",
            ContentType::with_params("*", "*", Some("charset=\"utf-8\";else=1")));
        assert_parse!("multipart/form-data; boundary=----WebKitFormBoundarypRshfItmvaC3aEuq",
            ContentType::with_params("multipart", "form-data",
                                     Some("boundary=----WebKitFormBoundarypRshfItmvaC3aEuq")));
    }

    #[test]
    fn test_bad_parses() {
        assert_no_parse!("application//json");
        assert_no_parse!("application///json");
        assert_no_parse!("*&_/*)()");
        assert_no_parse!("/json");
        assert_no_parse!("text/");
        assert_no_parse!("text//");
        assert_no_parse!("/");
        assert_no_parse!("*/");
        assert_no_parse!("/*");
        assert_no_parse!("///");
        assert_no_parse!("");
        assert_no_parse!("*/*;");
        assert_no_parse!("*/*;a=");
        assert_no_parse!("*/*;a= ");
        assert_no_parse!("*/*;a=@#$%^&*()");
        assert_no_parse!("*/*;;");
        assert_no_parse!("*/*;=;");
        assert_no_parse!("*/*=;");
        assert_no_parse!("*/*=;=");
        assert_no_parse!("*/*; a=b;");
        assert_no_parse!("*/*; a = b");
    }
}
