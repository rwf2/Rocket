use memchr::memchr2;

use http::RawStr;

/// Iterator over the key/value pairs of a given HTTP form string.
///
/// **Note:** The returned key/value pairs are _not_ URL decoded. To URL decode
/// the raw strings, use the
/// [`url_decode`](/rocket/http/struct.RawStr.html#method.url_decode) method:
///
/// ```rust
/// use rocket::request::{FormItems, FromFormValue};
///
/// let form_string = "greeting=Hello%2C+Mark%21&username=jake%2Fother";
/// for (key, value) in FormItems::from(form_string) {
///     let decoded_value = value.url_decode();
///     match key.as_str() {
///         "greeting" => assert_eq!(decoded_value, Ok("Hello, Mark!".into())),
///         "username" => assert_eq!(decoded_value, Ok("jake/other".into())),
///         _ => unreachable!()
///     }
/// }
/// ```
///
/// # Completion
///
/// The iterator keeps track of whether the form string was parsed to completion
/// to determine if the form string was malformed. The iterator can be queried
/// for completion via the [completed](#method.completed) method, which returns
/// `true` if the iterator parsed the entire string that was passed to it. The
/// iterator can also attempt to parse any remaining contents via
/// [exhaust](#method.exhaust); this method returns `true` if exhaustion
/// succeeded.
///
/// This iterator guarantees that all valid form strings are parsed to
/// completion. The iterator attempts to be lenient. In particular, it allows
/// the following oddball behavior:
///
///   * Trailing and consecutive `&` characters are allowed.
///   * Empty keys and/or values are allowed.
///
/// Additionally, the iterator skips items with both an empty key _and_ an empty
/// value: at least one of the two must be non-empty to be returned from this
/// iterator.
///
/// # Examples
///
/// `FormItems` can be used directly as an iterator:
///
/// ```rust
/// use rocket::request::FormItems;
///
/// // prints "greeting = hello", "username = jake", and "done = "
/// let form_string = "greeting=hello&username=jake&done";
/// for (key, value) in FormItems::from(form_string) {
///     println!("{} = {}", key, value);
/// }
/// ```
///
/// This is the same example as above, but the iterator is used explicitly.
///
/// ```rust
/// use rocket::request::FormItems;
///
/// let form_string = "greeting=hello&username=jake&done";
/// let mut items = FormItems::from(form_string);
///
/// let next = items.next().unwrap();
/// assert_eq!(next.0, "greeting");
/// assert_eq!(next.1, "hello");
///
/// let next = items.next().unwrap();
/// assert_eq!(next.0, "username");
/// assert_eq!(next.1, "jake");
///
/// let next = items.next().unwrap();
/// assert_eq!(next.0, "done");
/// assert_eq!(next.1, "");
///
/// assert_eq!(items.next(), None);
/// assert!(items.completed());
/// ```
pub struct FormItems<'f> {
    string: &'f RawStr,
    next_index: usize
}

impl<'f> FormItems<'f> {
    /// Returns `true` if the form string was parsed to completion. Returns
    /// `false` otherwise. All valid form strings will parse to completion,
    /// while invalid form strings will not.
    ///
    /// # Example
    ///
    /// A valid form string parses to completion:
    ///
    /// ```rust
    /// use rocket::request::FormItems;
    ///
    /// let mut items = FormItems::from("a=b&c=d");
    /// let key_values: Vec<_> = items.by_ref().collect();
    ///
    /// assert_eq!(key_values.len(), 2);
    /// assert_eq!(items.completed(), true);
    /// ```
    ///
    /// In invalid form string does not parse to completion:
    ///
    /// ```rust
    /// use rocket::request::FormItems;
    ///
    /// let mut items = FormItems::from("a=b&==d");
    /// let key_values: Vec<_> = items.by_ref().collect();
    ///
    /// assert_eq!(key_values.len(), 1);
    /// assert_eq!(items.completed(), false);
    /// ```
    #[inline]
    pub fn completed(&self) -> bool {
        self.next_index >= self.string.len()
    }

    /// Parses all remaining key/value pairs and returns `true` if parsing ran
    /// to completion. All valid form strings will parse to completion, while
    /// invalid form strings will not.
    ///
    /// # Example
    ///
    /// A valid form string can be exhausted:
    ///
    /// ```rust
    /// use rocket::request::FormItems;
    ///
    /// let mut items = FormItems::from("a=b&c=d");
    ///
    /// assert!(items.next().is_some());
    /// assert_eq!(items.completed(), false);
    /// assert_eq!(items.exhaust(), true);
    /// assert_eq!(items.completed(), true);
    /// ```
    ///
    /// An invalid form string cannot be exhausted:
    ///
    /// ```rust
    /// use rocket::request::FormItems;
    ///
    /// let mut items = FormItems::from("a=b&=d=");
    ///
    /// assert!(items.next().is_some());
    /// assert_eq!(items.completed(), false);
    /// assert_eq!(items.exhaust(), false);
    /// assert_eq!(items.completed(), false);
    /// assert!(items.next().is_none());
    /// ```
    #[inline]
    pub fn exhaust(&mut self) -> bool {
        while let Some(_) = self.next() {  }
        self.completed()
    }

    #[inline]
    #[doc(hidden)]
    pub fn mark_complete(&mut self) {
        self.next_index = self.string.len()
    }

    /// Retrieves the original string being parsed by this iterator. The string
    /// returned by this method does not change, regardless of the status of the
    /// iterator.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::FormItems;
    ///
    /// let form_string = "a=b&c=d";
    /// let mut items = FormItems::from(form_string);
    /// assert_eq!(items.inner_str(), form_string);
    ///
    /// assert!(items.next().is_some());
    /// assert_eq!(items.inner_str(), form_string);
    ///
    /// assert!(items.next().is_some());
    /// assert_eq!(items.inner_str(), form_string);
    ///
    /// assert!(items.next().is_none());
    /// assert_eq!(items.inner_str(), form_string);
    /// ```
    #[inline]
    pub fn inner_str(&self) -> &'f RawStr {
        self.string
    }
}

impl<'f> From<&'f RawStr> for FormItems<'f> {
    /// Returns an iterator over the key/value pairs in the
    /// `x-www-form-urlencoded` form `string`.
    #[inline(always)]
    fn from(string: &'f RawStr) -> FormItems<'f> {
        FormItems { string, next_index: 0 }
    }
}

impl<'f> From<&'f str> for FormItems<'f> {
    /// Returns an iterator over the key/value pairs in the
    /// `x-www-form-urlencoded` form `string`.
    #[inline(always)]
    fn from(string: &'f str) -> FormItems<'f> {
        FormItems::from(RawStr::from_str(string))
    }
}

impl<'f> Iterator for FormItems<'f> {
    type Item = (&'f RawStr, &'f RawStr);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let s = &self.string[self.next_index..];
            if s.is_empty() {
                return None;
            }

            let (key, rest, key_consumed) = match memchr2(b'=', b'&', s.as_bytes()) {
                Some(i) if s.as_bytes()[i] == b'=' => (&s[..i], &s[(i + 1)..], i + 1),
                Some(i) => (&s[..i], &s[i..], i),
                None => (s, &s[s.len()..], s.len())
            };

            let (value, val_consumed) = match memchr2(b'=', b'&', rest.as_bytes()) {
                Some(i) if rest.as_bytes()[i] == b'=' => return None,
                Some(i) => (&rest[..i], i + 1),
                None => (rest, rest.len())
            };

            self.next_index += key_consumed + val_consumed;
            match (key.is_empty(), value.is_empty()) {
                (true, true) => continue,
                _ => return Some((key.into(), value.into()))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::FormItems;

    macro_rules! check_form {
        (@bad $string:expr) => (check_form($string, None));
        ($string:expr, $expected:expr) => (check_form($string, Some($expected)));
    }

    fn check_form(string: &str, expected: Option<&[(&str, &str)]>) {
        let mut items = FormItems::from(string);
        let results: Vec<_> = items.by_ref().collect();
        if let Some(expected) = expected {
            assert_eq!(expected.len(), results.len(),
                "expected {:?}, got {:?} for {:?}", expected, results, string);

            for i in 0..results.len() {
                let (expected_key, actual_key) = (expected[i].0, results[i].0);
                let (expected_val, actual_val) = (expected[i].1, results[i].1);

                assert!(actual_key == expected_key,
                        "key [{}] mismatch for {}: expected {}, got {}",
                        i, string, expected_key, actual_key);

                assert!(actual_val == expected_val,
                        "val [{}] mismatch for {}: expected {}, got {}",
                        i, string, expected_val, actual_val);
            }
        } else {
            assert!(!items.exhaust(), "{} unexpectedly parsed successfully", string);
        }
    }

    #[test]
    fn test_form_string() {
        check_form!("username=user&password=pass",
                    &[("username", "user"), ("password", "pass")]);

        check_form!("user=user&user=pass", &[("user", "user"), ("user", "pass")]);
        check_form!("user=&password=pass", &[("user", ""), ("password", "pass")]);
        check_form!("user&password=pass", &[("user", ""), ("password", "pass")]);
        check_form!("foo&bar", &[("foo", ""), ("bar", "")]);

        check_form!("a=b", &[("a", "b")]);
        check_form!("value=Hello+World", &[("value", "Hello+World")]);

        check_form!("user=", &[("user", "")]);
        check_form!("user=&", &[("user", "")]);
        check_form!("a=b&a=", &[("a", "b"), ("a", "")]);
        check_form!("user=&password", &[("user", ""), ("password", "")]);
        check_form!("a=b&a", &[("a", "b"), ("a", "")]);

        check_form!("user=x&&", &[("user", "x")]);
        check_form!("user=x&&&&pass=word", &[("user", "x"), ("pass", "word")]);
        check_form!("user=x&&&&pass=word&&&x=z&d&&&e",
                    &[("user", "x"), ("pass", "word"), ("x", "z"), ("d", ""), ("e", "")]);

        check_form!("=&a=b&&=", &[("a", "b")]);
        check_form!("=b", &[("", "b")]);
        check_form!("=b&=c", &[("", "b"), ("", "c")]);

        check_form!("=", &[]);
        check_form!("&=&", &[]);
        check_form!("&", &[]);
        check_form!("=&=", &[]);

        check_form!(@bad "=b&==");
        check_form!(@bad "==");
        check_form!(@bad "=k=");
        check_form!(@bad "=abc=");
        check_form!(@bad "=abc=cd");
    }
}
