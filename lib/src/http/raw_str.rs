use std::ops::{Deref, DerefMut};
use std::borrow::Cow;
use std::convert::AsRef;
use std::cmp::Ordering;
use std::ascii::AsciiExt;
use std::str::Utf8Error;
use std::fmt;

use url;

use http::uncased::UncasedStr;

/// A reference to a raw HTTP string.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawStr(str);

impl RawStr {
    #[inline(always)]
    pub fn from_str<'a>(string: &'a str) -> &'a RawStr {
        string.into()
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self
    }

    #[inline(always)]
    pub fn as_uncased_str(&self) -> &UncasedStr {
        self.as_str().into()
    }

    /// Returns a URL-decoded version of the string. This is identical to
    /// percent decoding except that '+' characters are converted into spaces.
    /// This is the encoding used by form values.
    ///
    /// If the percent encoded values are not valid UTF-8, an `Err` is returned.
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
    #[inline]
    pub fn url_decode(&self) -> Result<String, Utf8Error> {
        let replaced = self.replace("+", " ");
        RawStr::from_str(replaced.as_str())
            .percent_decode()
            .map(|cow| cow.into_owned())
    }

    /// Returns a percent-decoded version of the string. If the percent encoded
    /// values are not valid UTF-8, an `Err` is returned.
    #[inline(always)]
    pub fn percent_decode(&self) -> Result<Cow<str>, Utf8Error> {
        url::percent_encoding::percent_decode(self.as_bytes()).decode_utf8()
    }

    /// Returns a percent-decoded version of the string. Any invalid UTF-8
    /// percent-encoded byte sequences will be replaced � U+FFFD, the
    /// replacement character.
    #[inline(always)]
    pub fn percent_decode_lossy(&self) -> Cow<str> {
        url::percent_encoding::percent_decode(self.as_bytes()).decode_utf8_lossy()
    }

    /// Do some HTML escaping.
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
