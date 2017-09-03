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
///   * A single trailing `&` character is allowed.
///   * Empty values are allowed.
///
/// # Examples
///
/// `FormItems` can be used directly as an iterator:
///
/// ```rust
/// use rocket::request::FormItems;
///
/// // prints "greeting = hello" then "username = jake"
/// let form_string = "greeting=hello&username=jake";
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
/// let form_string = "greeting=hello&username=jake";
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
    /// let mut items = FormItems::from("a=b&=d");
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
    /// let mut items = FormItems::from("a=b&=d");
    ///
    /// assert!(items.next().is_some());
    /// assert_eq!(items.completed(), false);
    /// assert_eq!(items.exhaust(), false);
    /// assert_eq!(items.completed(), false);
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
        FormItems {
            string: string,
            next_index: 0
        }
    }
}

impl<'f> From<&'f str> for FormItems<'f> {
    /// Returns an iterator over the key/value pairs in the
    /// `x-www-form-urlencoded` form `string`.
    #[inline(always)]
    fn from(string: &'f str) -> FormItems<'f> {
        FormItems {
            string: string.into(),
            next_index: 0
        }
    }
}

impl<'f> Iterator for FormItems<'f> {
    type Item = (&'f RawStr, &'f RawStr);

    fn next(&mut self) -> Option<Self::Item> {
        let s = &self.string[self.next_index..];
        let (key, rest) = match memchr2(b'=', b'&', s.as_bytes()) {
            Some(i) if s.as_bytes()[i] == b'=' => (&s[..i], &s[(i + 1)..]),
            Some(_) => return None,
            None => return None,
        };

        if key.is_empty() {
            return None;
        }

        let (value, consumed) = match rest.find('&') {
            Some(index) => (&rest[..index], index + 1),
            None => (rest, rest.len()),
        };

        self.next_index += key.len() + 1 + consumed;
        Some((key.into(), value.into()))
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
            assert_eq!(expected.len(), results.len());

            for i in 0..results.len() {
                let (expected_key, actual_key) = (expected[i].0, results[i].0);
                let (expected_val, actual_val) = (expected[i].1, results[i].1);

                assert!(actual_key == expected_key,
                        "key [{}] mismatch: expected {}, got {}",
                        i, expected_key, actual_key);

                assert!(actual_val == expected_val,
                        "val [{}] mismatch: expected {}, got {}",
                        i, expected_val, actual_val);
            }
        } else {
            assert!(!items.exhaust());
        }
    }

    #[test]
    fn test_form_string() {
        check_form!("username=user&password=pass",
                    &[("username", "user"), ("password", "pass")]);

        check_form!("user=user&user=pass",
                    &[("user", "user"), ("user", "pass")]);

        check_form!("user=&password=pass",
                    &[("user", ""), ("password", "pass")]);

        check_form!("a=b", &[("a", "b")]);
        check_form!("value=Hello+World", &[("value", "Hello+World")]);

        check_form!("user=", &[("user", "")]);
        check_form!("user=&", &[("user", "")]);
        check_form!("a=b&a=", &[("a", "b"), ("a", "")]);

        check_form!(@bad "user=&password");
        check_form!(@bad "user=x&&");
        check_form!(@bad "a=b&a");
        check_form!(@bad "=");
        check_form!(@bad "&");
        check_form!(@bad "=&");
        check_form!(@bad "&=&");
        check_form!(@bad "=&=");
    }
}
