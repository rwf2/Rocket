//! Contains types that encapsulate uncased ASCII strings.
//!
//! An 'uncased' ASCII string is case-preserving. That is, the string itself
//! contains cased characters, but comparison (including ordering, equality, and
//! hashing) is case-insensitive.

use std::ops::Deref;
use std::borrow::{Cow, Borrow};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::fmt;

/// A reference to an uncased (case-preserving) ASCII string. This is typically
/// created from an `&str` as follows:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::uncased::UncasedStr;
///
/// let ascii_ref: &UncasedStr = "Hello, world!".into();
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct UncasedStr(str);

impl UncasedStr {
    /// Returns a reference to an `UncasedStr` from an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::UncasedStr;
    ///
    /// let uncased_str = UncasedStr::new("Hello!");
    /// assert_eq!(uncased_str, "hello!");
    /// assert_eq!(uncased_str, "Hello!");
    /// assert_eq!(uncased_str, "HeLLo!");
    /// ```
    #[inline(always)]
    pub fn new(string: &str) -> &UncasedStr {
        // This is simply a `newtype`-like transformation. The `repr(C)` ensures
        // that this is safe and correct. Note this exact pattern appears often
        // in the standard library.
        unsafe { &*(string as *const str as *const UncasedStr) }
    }

    /// Returns `self` as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::UncasedStr;
    ///
    /// let uncased_str = UncasedStr::new("Hello!");
    /// assert_eq!(uncased_str.as_str(), "Hello!");
    /// assert_ne!(uncased_str.as_str(), "hELLo!");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts a `Box<UncasedStr>` into an `Uncased` without copying or allocating.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::Uncased;
    ///
    /// let uncased = Uncased::new("Hello!");
    /// let boxed = uncased.clone().into_boxed_uncased();
    /// assert_eq!(boxed.into_uncased(), uncased);
    /// ```
    #[inline(always)]
    pub fn into_uncased(self: Box<UncasedStr>) -> Uncased<'static> {
        // This is the inverse of a `newtype`-like transformation. The `repr(C)`
        // ensures that this is safe and correct. Note this exact pattern
        // appears often in the standard library.
        unsafe {
            let raw_str = Box::into_raw(self) as *mut str;
            Uncased::from(Box::from_raw(raw_str).into_string())
        }
    }
}

impl PartialEq for UncasedStr {
    #[inline(always)]
    fn eq(&self, other: &UncasedStr) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl PartialEq<str> for UncasedStr {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<UncasedStr> for str {
    #[inline(always)]
    fn eq(&self, other: &UncasedStr) -> bool {
        other.0.eq_ignore_ascii_case(self)
    }
}

impl<'a> PartialEq<&'a str> for UncasedStr {
    #[inline(always)]
    fn eq(&self, other: & &'a str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl<'a> PartialEq<UncasedStr> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &UncasedStr) -> bool {
        other.0.eq_ignore_ascii_case(self)
    }
}

impl<'a> From<&'a str> for &'a UncasedStr {
    #[inline(always)]
    fn from(string: &'a str) -> &'a UncasedStr {
        UncasedStr::new(string)
    }
}

impl Eq for UncasedStr {  }

impl Hash for UncasedStr {
    #[inline(always)]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        for byte in self.0.bytes() {
            hasher.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl PartialOrd for UncasedStr {
    #[inline(always)]
    fn partial_cmp(&self, other: &UncasedStr) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UncasedStr {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_chars = self.0.chars().map(|c| c.to_ascii_lowercase());
        let other_chars = other.0.chars().map(|c| c.to_ascii_lowercase());
        self_chars.cmp(other_chars)
    }
}

impl fmt::Display for UncasedStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An uncased (case-preserving), owned _or_ borrowed ASCII string.
#[derive(Clone, Debug)]
pub struct Uncased<'s> {
    #[doc(hidden)]
    pub string: Cow<'s, str>
}

impl<'s> Uncased<'s> {
    /// Creates a new `Uncased` string from `string` without allocating.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::Uncased;
    ///
    /// let uncased = Uncased::new("Content-Type");
    /// assert_eq!(uncased, "content-type");
    /// assert_eq!(uncased, "CONTENT-Type");
    /// ```
    #[inline(always)]
    pub fn new<S: Into<Cow<'s, str>>>(string: S) -> Uncased<'s> {
        Uncased { string: string.into() }
    }

    /// Converts `self` into an owned `String`, allocating if necessary.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::Uncased;
    ///
    /// let uncased = Uncased::new("Content-Type");
    /// let string = uncased.into_string();
    /// assert_eq!(string, "Content-Type".to_string());
    /// ```
    #[inline(always)]
    pub fn into_string(self) -> String {
        self.string.into_owned()
    }

    /// Converts `self` into a `Box<UncasedStr>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uncased::Uncased;
    ///
    /// let boxed = Uncased::new("Content-Type").into_boxed_uncased();
    /// assert_eq!(&*boxed, "content-type");
    /// ```
    #[inline(always)]
    pub fn into_boxed_uncased(self) -> Box<UncasedStr> {
        // This is simply a `newtype`-like transformation. The `repr(C)` ensures
        // that this is safe and correct. Note this exact pattern appears often
        // in the standard library.
        unsafe {
            let raw_str = Box::into_raw(self.string.into_owned().into_boxed_str());
            Box::from_raw(raw_str as *mut UncasedStr)
        }
    }

    /// Returns the inner `Cow`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn into_cow(self) -> Cow<'s, str> {
        self.string
    }
}

impl<'a> Deref for Uncased<'a> {
    type Target = UncasedStr;

    #[inline(always)]
    fn deref(&self) -> &UncasedStr {
        UncasedStr::new(self.string.borrow())
    }
}

impl<'a> AsRef<UncasedStr> for Uncased<'a>{
    #[inline(always)]
    fn as_ref(&self) -> &UncasedStr {
        UncasedStr::new(self.string.borrow())
    }
}

impl<'a> Borrow<UncasedStr> for Uncased<'a> {
    #[inline(always)]
    fn borrow(&self) -> &UncasedStr {
        self.as_str().into()
    }
}

impl<'s, 'c: 's> From<&'c str> for Uncased<'s> {
    #[inline(always)]
    fn from(string: &'c str) -> Self {
        Uncased::new(string)
    }
}

impl From<String> for Uncased<'static> {
    #[inline(always)]
    fn from(string: String) -> Self {
        Uncased::new(string)
    }
}

impl<'s, 'c: 's> From<Cow<'c, str>> for Uncased<'s> {
    #[inline(always)]
    fn from(string: Cow<'c, str>) -> Self {
        Uncased::new(string)
    }
}

impl<'s, 'c: 's, T: Into<Cow<'c, str>>> From<T> for Uncased<'s> {
    #[inline(always)]
    default fn from(string: T) -> Self {
        Uncased::new(string)
    }
}

impl<'a, 'b> PartialOrd<Uncased<'b>> for Uncased<'a> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Uncased<'b>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<'a> Ord for Uncased<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<'s> fmt::Display for Uncased<'s> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.string.fmt(f)
    }
}

impl<'a, 'b> PartialEq<Uncased<'b>> for Uncased<'a> {
    #[inline(always)]
    fn eq(&self, other: &Uncased<'b>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<'a> PartialEq<str> for Uncased<'a> {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_ref().eq(other)
    }
}

impl<'b> PartialEq<Uncased<'b>> for str {
    #[inline(always)]
    fn eq(&self, other: &Uncased<'b>) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'a, 'b> PartialEq<&'b str> for Uncased<'a> {
    #[inline(always)]
    fn eq(&self, other: & &'b str) -> bool {
        self.as_ref().eq(other)
    }
}

impl<'a, 'b> PartialEq<Uncased<'b>> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &Uncased<'b>) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'s> Eq for Uncased<'s> {  }

impl<'s> Hash for Uncased<'s> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_ref().hash(hasher)
    }
}

/// Returns true if `s1` and `s2` are equal without considering case.
///
/// That is, for ASCII strings, this function returns `s1.to_lower() ==
/// s2.to_lower()`, but does it in a much faster way.
#[inline(always)]
pub fn uncased_eq<S1: AsRef<str>, S2: AsRef<str>>(s1: S1, s2: S2) -> bool {
    UncasedStr::new(s1.as_ref()) == UncasedStr::new(s2.as_ref())
}

#[cfg(test)]
mod tests {
    use super::Uncased;
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
                    let ascii_a = Uncased::from(str_a);
                    let ascii_b = Uncased::from(str_b);
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
        assert!(Uncased::from("foobar") == Uncased::from("FOOBAR"));
        assert!(Uncased::from("a") == Uncased::from("A"));

        assert!(Uncased::from("a") < Uncased::from("B"));
        assert!(Uncased::from("A") < Uncased::from("B"));
        assert!(Uncased::from("A") < Uncased::from("b"));

        assert!(Uncased::from("aa") > Uncased::from("a"));
        assert!(Uncased::from("aa") > Uncased::from("A"));
        assert!(Uncased::from("AA") > Uncased::from("a"));
        assert!(Uncased::from("AA") > Uncased::from("a"));
        assert!(Uncased::from("Aa") > Uncased::from("a"));
        assert!(Uncased::from("Aa") > Uncased::from("A"));
        assert!(Uncased::from("aA") > Uncased::from("a"));
        assert!(Uncased::from("aA") > Uncased::from("A"));
    }
}
