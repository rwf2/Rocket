//! Contains types that encapsulate uncased ASCII strings.
//!
//! An 'uncased' ASCII string is case-preserving. That is, the string itself
//! contains cased charaters, but comparison (including ordering, equaility, and
//! hashing) is case-insensitive.

use std::ops::Deref;
use std::borrow::{Cow, Borrow};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ascii::AsciiExt;
use std::fmt;

/// A reference to an uncased (case-preserving) ASCII string. This is typically
/// created from an `&str` as follows:
///
/// ```rust,ignore
/// use rocket::http::ascii::UncasedAsciiRef;
///
/// let ascii_ref: &UncasedAsciiRef = "Hello, world!".into();
/// ```
#[derive(Debug)]
pub struct UncasedAsciiRef(str);

impl PartialEq for UncasedAsciiRef {
    #[inline(always)]
    fn eq(&self, other: &UncasedAsciiRef) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl PartialEq<str> for UncasedAsciiRef {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<UncasedAsciiRef> for str {
    #[inline(always)]
    fn eq(&self, other: &UncasedAsciiRef) -> bool {
        other.0.eq_ignore_ascii_case(self)
    }
}

impl<'a> PartialEq<&'a str> for UncasedAsciiRef {
    #[inline(always)]
    fn eq(&self, other: & &'a str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl<'a> PartialEq<UncasedAsciiRef> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &UncasedAsciiRef) -> bool {
        other.0.eq_ignore_ascii_case(self)
    }
}

impl<'a> From<&'a str> for &'a UncasedAsciiRef {
    #[inline(always)]
    fn from(string: &'a str) -> &'a UncasedAsciiRef {
        unsafe { ::std::mem::transmute(string) }
    }
}

impl Eq for UncasedAsciiRef {  }

impl Hash for UncasedAsciiRef {
    #[inline(always)]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        for byte in self.0.bytes() {
            hasher.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl PartialOrd for UncasedAsciiRef {
    #[inline(always)]
    fn partial_cmp(&self, other: &UncasedAsciiRef) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UncasedAsciiRef {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_chars = self.0.chars().map(|c| c.to_ascii_lowercase());
        let other_chars = other.0.chars().map(|c| c.to_ascii_lowercase());
        self_chars.cmp(other_chars)
    }
}

/// An uncased (case-preserving) ASCII string.
#[derive(Clone, Debug)]
pub struct UncasedAscii<'s> {
    pub string: Cow<'s, str>
}

impl<'s> UncasedAscii<'s> {
    /// Creates a new UncaseAscii string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rocket::http::ascii::UncasedAscii;
    ///
    /// let uncased_ascii = UncasedAScii::new("Content-Type");
    /// ```
    #[inline(always)]
    pub fn new<S: Into<Cow<'s, str>>>(string: S) -> UncasedAscii<'s> {
        UncasedAscii { string: string.into() }
    }

    /// Converts `self` into an owned `String`, allocating if necessary,
    #[inline(always)]
    pub fn into_string(self) -> String {
        self.string.into_owned()
    }

    /// Borrows the inner string.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.string.borrow()
    }

    /// Returns the inner `Cow`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn into_cow(self) -> Cow<'s, str> {
        self.string
    }
}

impl<'a> Deref for UncasedAscii<'a> {
    type Target = UncasedAsciiRef;

    #[inline(always)]
    fn deref(&self) -> &UncasedAsciiRef {
        self.as_str().into()
    }
}

impl<'a> AsRef<UncasedAsciiRef> for UncasedAscii<'a>{
    #[inline(always)]
    fn as_ref(&self) -> &UncasedAsciiRef {
        self.as_str().into()
    }
}

impl<'a> Borrow<UncasedAsciiRef> for UncasedAscii<'a> {
    #[inline(always)]
    fn borrow(&self) -> &UncasedAsciiRef {
        self.as_str().into()
    }
}

impl<'s, 'c: 's> From<&'c str> for UncasedAscii<'s> {
    #[inline(always)]
    fn from(string: &'c str) -> Self {
        UncasedAscii::new(string)
    }
}

impl From<String> for UncasedAscii<'static> {
    #[inline(always)]
    fn from(string: String) -> Self {
        UncasedAscii::new(string)
    }
}

impl<'s, 'c: 's> From<Cow<'c, str>> for UncasedAscii<'s> {
    #[inline(always)]
    fn from(string: Cow<'c, str>) -> Self {
        UncasedAscii::new(string)
    }
}

impl<'s, 'c: 's, T: Into<Cow<'c, str>>> From<T> for UncasedAscii<'s> {
    #[inline(always)]
    default fn from(string: T) -> Self {
        UncasedAscii::new(string)
    }
}

impl<'a, 'b> PartialOrd<UncasedAscii<'b>> for UncasedAscii<'a> {
    #[inline(always)]
    fn partial_cmp(&self, other: &UncasedAscii<'b>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<'a> Ord for UncasedAscii<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<'s> fmt::Display for UncasedAscii<'s> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.string.fmt(f)
    }
}

impl<'a, 'b> PartialEq<UncasedAscii<'b>> for UncasedAscii<'a> {
    #[inline(always)]
    fn eq(&self, other: &UncasedAscii<'b>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<'a> PartialEq<str> for UncasedAscii<'a> {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_ref().eq(other)
    }
}

impl<'b> PartialEq<UncasedAscii<'b>> for str {
    #[inline(always)]
    fn eq(&self, other: &UncasedAscii<'b>) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'a, 'b> PartialEq<&'b str> for UncasedAscii<'a> {
    #[inline(always)]
    fn eq(&self, other: & &'b str) -> bool {
        self.as_ref().eq(other)
    }
}

impl<'a, 'b> PartialEq<UncasedAscii<'b>> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &UncasedAscii<'b>) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'s> Eq for UncasedAscii<'s> {  }

impl<'s> Hash for UncasedAscii<'s> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_ref().hash(hasher)
    }
}

/// Returns true if `s1` and `s2` are equal without considering case. That is,
/// for ASCII strings, this function returns s1.to_lower() == s2.to_lower(), but
/// does it in a much faster way.
#[inline(always)]
pub fn uncased_eq<S1: AsRef<str>, S2: AsRef<str>>(s1: S1, s2: S2) -> bool {
    let ascii_ref_1: &UncasedAsciiRef = s1.as_ref().into();
    let ascii_ref_2: &UncasedAsciiRef = s2.as_ref().into();
    ascii_ref_1 == ascii_ref_2
}

#[cfg(test)]
mod tests {
    use super::UncasedAscii;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    macro_rules! assert_uncased_eq {
        ($($string:expr),+) => ({
            let mut strings = Vec::new();
            $(strings.push($string);)+

            for i in 0..strings.len() {
                for j in i..strings.len() {
                    let (str_a, str_b) = (strings[i], strings[j]);
                    let ascii_a = UncasedAscii::from(str_a);
                    let ascii_b = UncasedAscii::from(str_b);
                    assert_eq!(ascii_a, ascii_b);
                    assert_eq!(hash(&ascii_a), hash(&ascii_b));
                    assert_eq!(ascii_a, str_a);
                    assert_eq!(ascii_b, str_b);
                    assert_eq!(ascii_a, str_b);
                    assert_eq!(ascii_b, str_a);
                }
            }
        })
    }

    #[test]
    fn test_case_insensitive() {
        assert_uncased_eq!["a", "A"];
        assert_uncased_eq!["foobar", "FOOBAR", "FooBar", "fOObAr", "fooBAR"];
        assert_uncased_eq!["", ""];
        assert_uncased_eq!["content-type", "Content-Type", "CONTENT-TYPE"];
    }

    #[test]
    fn test_case_cmp() {
        assert!(UncasedAscii::from("foobar") == UncasedAscii::from("FOOBAR"));
        assert!(UncasedAscii::from("a") == UncasedAscii::from("A"));

        assert!(UncasedAscii::from("a") < UncasedAscii::from("B"));
        assert!(UncasedAscii::from("A") < UncasedAscii::from("B"));
        assert!(UncasedAscii::from("A") < UncasedAscii::from("b"));

        assert!(UncasedAscii::from("aa") > UncasedAscii::from("a"));
        assert!(UncasedAscii::from("aa") > UncasedAscii::from("A"));
        assert!(UncasedAscii::from("AA") > UncasedAscii::from("a"));
        assert!(UncasedAscii::from("AA") > UncasedAscii::from("a"));
        assert!(UncasedAscii::from("Aa") > UncasedAscii::from("a"));
        assert!(UncasedAscii::from("Aa") > UncasedAscii::from("A"));
        assert!(UncasedAscii::from("aA") > UncasedAscii::from("a"));
        assert!(UncasedAscii::from("aA") > UncasedAscii::from("A"));
    }
}
