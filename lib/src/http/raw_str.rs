use std::ops::{Deref, DerefMut};
use std::borrow::Cow;
use std::convert::AsRef;
use std::cmp::Ordering;
use std::ascii::AsciiExt;
use std::str::Utf8Error;
use std::fmt;

use url;

use http::uncased::UncasedStr;

/// A reference to a string inside of a raw HTTP message.
///
/// A `RawStr` is an unsanitzed, unvalidated, and undecoded raw string from an
/// HTTP message. It exists to separate validated string inputs, represented by
/// the `String`, `&str`, and `Cow<str>` types, from unvalidated inputs,
/// represented by `&RawStr`.
///
/// # Validation
///
/// An `&RawStr` should be converted into one of the validated string input
/// types through methods on `RawStr`. These methods are summarized below:
///
///   * **[`url_decode`]** - used to decode a raw string in a form value context
///   * **[`percent_decode`], [`percent_decode_lossy`]** - used to
///   percent-decode a raw string, typically in a URL context
///   * **[`html_escape`]** - used to decode a string for use in HTML templates
///   * **[`as_str`]** - used when the `RawStr` is known to be safe in the
///   context of its intended use. Use sparingly and with care!
///   * **[`as_uncased_str`]** - used when the `RawStr` is known to be safe in
///   the context of its intended, uncased use
///
/// [`as_str`]: /rocket/http/struct.RawStr.html#method.as_str
/// [`as_uncased_str`]: /rocket/http/struct.RawStr.html#method.as_uncased_str
/// [`url_decode`]: /rocket/http/struct.RawStr.html#method.url_decode
/// [`html_escape`]: /rocket/http/struct.RawStr.html#method.html_escape
/// [`percent_decode`]: /rocket/http/struct.RawStr.html#method.percent_decode
/// [`percent_decode_lossy`]: /rocket/http/struct.RawStr.html#method.percent_decode_lossy
///
/// # Usage
///
/// A `RawStr` is a dynamically sized type (just like `str`). It is always used
/// through a reference an as `&RawStr` (just like &str). You'll likely
/// encounted an `&RawStr` as a parameter via [`FromParam`] or as a form value
/// via [`FromFormValue`].
///
/// [`FromParam`]: /rocket/request/trait.FromParam.html
/// [`FromFormValue`]: /rocket/request/trait.FromFormValue.html
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawStr(str);

impl RawStr {
    /// Constructs an `&RawStr` from an `&str` at no cost.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Hello, world!");
    ///
    /// // `into` can also be used; note that the type must be specified
    /// let raw_str: &RawStr = "Hello, world!".into();
    /// ```
    #[inline(always)]
    pub fn from_str<'a>(string: &'a str) -> &'a RawStr {
        string.into()
    }

    /// Returns a percent-decoded version of the string.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the percent encoded values are not valid UTF-8.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Hello%21");
    /// let decoded = raw_str.percent_decode();
    /// assert_eq!(decoded, Ok("Hello!".into()));
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { ::std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::from_str(bad_str);
    /// assert!(bad_raw_str.percent_decode().is_err());
    /// ```
    #[inline(always)]
    pub fn percent_decode(&self) -> Result<Cow<str>, Utf8Error> {
        url::percent_encoding::percent_decode(self.as_bytes()).decode_utf8()
    }

    /// Returns a percent-decoded version of the string. Any invalid UTF-8
    /// percent-encoded byte sequences will be replaced � U+FFFD, the
    /// replacement character.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the percent encoded values are not valid UTF-8.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Hello%21");
    /// let decoded = raw_str.percent_decode_lossy();
    /// assert_eq!(decoded, "Hello!");
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { ::std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::from_str(bad_str);
    /// assert_eq!(bad_raw_str.percent_decode_lossy(), "a=�");
    /// ```
    #[inline(always)]
    pub fn percent_decode_lossy(&self) -> Cow<str> {
        url::percent_encoding::percent_decode(self.as_bytes()).decode_utf8_lossy()
    }

    /// Returns a URL-decoded version of the string. This is identical to
    /// percent decoding except that `+` characters are converted into spaces.
    /// This is the encoding used by form values.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the percent encoded values are not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "Hello%2C+world%21".into();
    /// let decoded = raw_str.url_decode();
    /// assert_eq!(decoded, Ok("Hello, world!".to_string()));
    /// ```
    pub fn url_decode(&self) -> Result<String, Utf8Error> {
        let replaced = self.replace("+", " ");
        RawStr::from_str(replaced.as_str())
            .percent_decode()
            .map(|cow| cow.into_owned())
    }

    /// Returns an HTML escaped version of `self`. Allocates only when
    /// characters need to be escaped.
    ///
    /// The following characters are escaped: `&`, `<`, `>`, `"`, `'`, `/`,
    /// <code>`</code>. **This suffices as long as the escaped string is not
    /// used in an execution context such as inside of &lt;script> or &lt;style>
    /// tags!** See the [OWASP XSS Prevention Rules] for more information.
    ///
    /// [OWASP XSS Prevention Rules]: https://www.owasp.org/index.php/XSS_%28Cross_Site_Scripting%29_Prevention_Cheat_Sheet#XSS_Prevention_Rules
    ///
    /// # Example
    ///
    /// Strings with HTML sequences are escaped:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "<b>Hi!</b>".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "&lt;b&gt;Hi!&lt;&#x2F;b&gt;");
    ///
    /// let raw_str: &RawStr = "Hello, <i>world!</i>".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "Hello, &lt;i&gt;world!&lt;&#x2F;i&gt;");
    /// ```
    ///
    /// Strings without HTML sequences remain untouched:
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "Hello!".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "Hello!");
    ///
    /// let raw_str: &RawStr = "大阪".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "大阪");
    /// ```
    pub fn html_escape(&self) -> Cow<str> {
        let mut escaped = false;
        let mut allocated = Vec::new(); // this is allocation free
        for c in self.as_bytes() {
            match *c {
                b'&' | b'<' | b'>' | b'"' | b'\'' | b'/' | b'`' => {
                    if !escaped {
                        let i = (c as *const u8 as usize) - (self.as_ptr() as usize);
                        allocated = Vec::with_capacity(self.len() * 2);
                        allocated.extend_from_slice(&self.as_bytes()[..i]);
                    }

                    match *c {
                        b'&' => allocated.extend_from_slice(b"&amp;"),
                        b'<' => allocated.extend_from_slice(b"&lt;"),
                        b'>' => allocated.extend_from_slice(b"&gt;"),
                        b'"' => allocated.extend_from_slice(b"&quot;"),
                        b'\'' => allocated.extend_from_slice(b"&#x27;"),
                        b'/' => allocated.extend_from_slice(b"&#x2F;"),
                        // Old versions of IE treat a ` as a '.
                        b'`' => allocated.extend_from_slice(b"&#96;"),
                        _ => unreachable!()
                    }

                    escaped = true;
                }
                _ if escaped => allocated.push(*c),
                _ => {  }
            }
        }

        if escaped {
            unsafe { Cow::Owned(String::from_utf8_unchecked(allocated)) }
        } else {
            Cow::Borrowed(self.as_str())
        }
    }

    /// Converts `self` into an `&str`.
    ///
    /// This method should be used sparingly. **Only use this method when you
    /// are absolutely certain that doing so is safe.**
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Hello, world!");
    /// assert_eq!(raw_str.as_str(), "Hello, world!");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Converts `self` into an `&UncasedStr`.
    ///
    /// This method should be used sparingly. **Only use this method when you
    /// are absolutely certain that doing so is safe.**
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Content-Type");
    /// assert!(raw_str.as_uncased_str() == "content-TYPE");
    /// ```
    #[inline(always)]
    pub fn as_uncased_str(&self) -> &UncasedStr {
        self.as_str().into()
    }
}

impl<'a> From<&'a str> for &'a RawStr {
    #[inline(always)]
    fn from(string: &'a str) -> &'a RawStr {
        unsafe { ::std::mem::transmute(string) }
    }
}

impl PartialEq<str> for RawStr {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for RawStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<String> for &'a RawStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd<str> for RawStr {
    #[inline(always)]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        (self as &str).partial_cmp(other)
    }
}

impl AsRef<str> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToString for RawStr {
    #[inline(always)]
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

impl AsciiExt for RawStr {
    type Owned = String;

    #[inline(always)]
    fn is_ascii(&self) -> bool { (self as &str).is_ascii() }

    #[inline(always)]
    fn to_ascii_uppercase(&self) -> String { (self as &str).to_ascii_uppercase() }

    #[inline(always)]
    fn to_ascii_lowercase(&self) -> String { (self as &str).to_ascii_lowercase() }

    #[inline(always)]
    fn make_ascii_uppercase(&mut self) { (self as &mut str).make_ascii_uppercase() }

    #[inline(always)]
    fn make_ascii_lowercase(&mut self) { (self as &mut str).make_ascii_lowercase() }

    #[inline(always)]
    fn eq_ignore_ascii_case(&self, o: &RawStr) -> bool {
        (self as &str).eq_ignore_ascii_case(o as &str)
    }
}

impl Deref for RawStr {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl DerefMut for RawStr {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl fmt::Display for RawStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::RawStr;

    #[test]
    fn can_compare() {
        let raw_str = RawStr::from_str("abc");
        assert_eq!(raw_str, "abc");
        assert_eq!("abc", raw_str.as_str());
        assert_eq!(raw_str, RawStr::from_str("abc"));
        assert_eq!(raw_str, "abc".to_string());
        assert_eq!("abc".to_string(), raw_str.as_str());
    }
}
