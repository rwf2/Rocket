use memchr::memchr2;

use crate::http::RawStr;

/// Iterator over the key/value pairs of a given HTTP form string.
///
/// ```rust
/// use rocket::request::{FormItems, FromFormValue};
///
/// // Using the `key_value_decoded` method of `FormItem`.
/// let form_string = "greeting=Hello%2C+Mark%21&username=jake%2Fother";
/// for (key, value) in FormItems::from(form_string).map(|i| i.key_value_decoded()) {
///     match &*key {
///         "greeting" => assert_eq!(value, "Hello, Mark!".to_string()),
///         "username" => assert_eq!(value, "jake/other".to_string()),
///         _ => unreachable!()
///     }
/// }
///
/// // Accessing the fields of `FormItem` directly, including `raw`.
/// for item in FormItems::from(form_string) {
///     match item.key.as_str() {
///         "greeting" => {
///             assert_eq!(item.raw, "greeting=Hello%2C+Mark%21");
///             assert_eq!(item.value, "Hello%2C+Mark%21");
///             assert_eq!(item.value.url_decode(), Ok("Hello, Mark!".into()));
///         }
///         "username" => {
///             assert_eq!(item.raw, "username=jake%2Fother");
///             assert_eq!(item.value, "jake%2Fother");
///             assert_eq!(item.value.url_decode(), Ok("jake/other".into()));
///         }
///         _ => unreachable!()
///     }
/// }
/// ```
///
/// # Form Items via. `FormItem`
///
/// This iterator returns values of the type [`FormItem`]. To access the
/// associated key/value pairs of the form item, either directly access them via
/// the [`key`](FormItem.key) and [`value`](FormItem.value) fields, use the
/// [`FormItem::key_value()`] method to get a tuple of the _raw_ `(key, value)`,
/// or use the [`key_value_decoded()`](FormItem::key_value_decoded()) method to
/// get a tuple of the decoded (`key`, `value`).
///
/// # Completion
///
/// The iterator keeps track of whether the form string was parsed to completion
/// to determine if the form string was malformed. The iterator can be queried
/// for completion via the [`completed()`](#method.completed) method, which
/// returns `true` if the iterator parsed the entire string that was passed to
/// it. The iterator can also attempt to parse any remaining contents via
/// [`exhaust()`](#method.exhaust); this method returns `true` if exhaustion
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
/// for (key, value) in FormItems::from(form_string).map(|item| item.key_value()) {
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
/// assert_eq!(next.key, "greeting");
/// assert_eq!(next.value, "hello");
///
/// let next = items.next().unwrap();
/// assert_eq!(next.key, "username");
/// assert_eq!(next.value, "jake");
///
/// let next = items.next().unwrap();
/// assert_eq!(next.key, "done");
/// assert_eq!(next.value, "");
///
/// assert_eq!(items.next(), None);
/// assert!(items.completed());
/// ```
#[derive(Debug)]
pub enum FormItems<'f> {
    #[doc(hidden)]
    Raw {
        string: &'f RawStr,
        next_index: usize
    },
    #[doc(hidden)]
    Cooked {
        items: &'f [FormItem<'f>],
        next_index: usize
    }
}

/// A form items returned by the [`FormItems`] iterator.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FormItem<'f> {
    /// The full, nonempty string for the item, not including `&` delimiters.
    pub raw: &'f RawStr,
    /// The key for the item, which may be empty if `value` is nonempty.
    ///
    /// **Note:** The key is _not_ URL decoded. To URL decode the raw strings,
    /// use the [`RawStr::url_decode()`] method or access key-value pairs with
    /// [`key_value_decoded()`](FormItem::key_value_decoded()).
    pub key: &'f RawStr,
    /// The value for the item, which may be empty if `key` is nonempty.
    ///
    /// **Note:** The value is _not_ URL decoded. To URL decode the raw strings,
    /// use the [`RawStr::url_decode()`] method or access key-value pairs with
    /// [`key_value_decoded()`](FormItem::key_value_decoded()).
    pub value: &'f RawStr
}

impl<'f> FormItem<'f> {
    /// Extracts the raw `key` and `value` as a tuple.
    ///
    /// This is equivalent to `(item.key, item.value)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::FormItem;
    ///
    /// let item = FormItem {
    ///     raw: "hello=%2C+world%21".into(),
    ///     key: "hello".into(),
    ///     value: "%2C+world%21".into(),
    /// };
    ///
    /// let (key, value) = item.key_value();
    /// assert_eq!(key, "hello");
    /// assert_eq!(value, "%2C+world%21");
    /// ```
    #[inline(always)]
    pub fn key_value(&self) -> (&'f RawStr, &'f RawStr) {
        (self.key, self.value)
    }

    /// Extracts and lossy URL decodes the `key` and `value` as a tuple.
    ///
    /// This is equivalent to `(item.key.url_decode_lossy(),
    /// item.value.url_decode_lossy)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::FormItem;
    ///
    /// let item = FormItem {
    ///     raw: "hello=%2C+world%21".into(),
    ///     key: "hello".into(),
    ///     value: "%2C+world%21".into(),
    /// };
    ///
    /// let (key, value) = item.key_value_decoded();
    /// assert_eq!(key, "hello");
    /// assert_eq!(value, ", world!");
    /// ```
    #[inline(always)]
    pub fn key_value_decoded(&self) -> (String, String) {
        (self.key.url_decode_lossy(), self.value.url_decode_lossy())
    }

    /// Extracts `raw` and the raw `key` and `value` as a triple.
    ///
    /// This is equivalent to `(item.raw, item.key, item.value)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::FormItem;
    ///
    /// let item = FormItem {
    ///     raw: "hello=%2C+world%21".into(),
    ///     key: "hello".into(),
    ///     value: "%2C+world%21".into(),
    /// };
    ///
    /// let (raw, key, value) = item.explode();
    /// assert_eq!(raw, "hello=%2C+world%21");
    /// assert_eq!(key, "hello");
    /// assert_eq!(value, "%2C+world%21");
    /// ```
    #[inline(always)]
    pub fn explode(&self) -> (&'f RawStr, &'f RawStr, &'f RawStr) {
        (self.raw, self.key, self.value)
    }
}

impl FormItems<'_> {
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
        match self {
            FormItems::Raw { string, next_index } => *next_index >= string.len(),
            FormItems::Cooked { items, next_index } => *next_index >= items.len(),
        }
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
        match self {
            FormItems::Raw { string, ref mut next_index } => *next_index = string.len(),
            FormItems::Cooked { items, ref mut next_index } => *next_index = items.len(),
        }
    }
}

impl<'f> From<&'f RawStr> for FormItems<'f> {
    #[inline(always)]
    fn from(string: &'f RawStr) -> FormItems<'f> {
        FormItems::Raw { string, next_index: 0 }
    }
}

impl<'f> From<&'f str> for FormItems<'f> {
    #[inline(always)]
    fn from(string: &'f str) -> FormItems<'f> {
        FormItems::from(RawStr::from_str(string))
    }
}

impl<'f> From<&'f [FormItem<'f>]> for FormItems<'f> {
    #[inline(always)]
    fn from(items: &'f [FormItem<'f>]) -> FormItems<'f> {
        FormItems::Cooked { items, next_index: 0 }
    }
}

fn raw<'f>(string: &mut &'f RawStr, index: &mut usize) -> Option<FormItem<'f>> {
    loop {
        let start = *index;
        let s = &string[start..];
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

        *index += key_consumed + val_consumed;
        let raw = &string[start..(start + key_consumed + value.len())];
        match (key.is_empty(), value.is_empty()) {
            (true, true) => continue,
            _ => return Some(FormItem {
                raw: raw.into(),
                key: key.into(),
                value: value.into()
            })
        }
    }
}

impl<'f> Iterator for FormItems<'f> {
    type Item = FormItem<'f>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FormItems::Raw { ref mut string, ref mut next_index } => {
                raw(string, next_index)
            }
            FormItems::Cooked { items, ref mut next_index } => {
                if *next_index < items.len() {
                    let item = items[*next_index];
                    *next_index += 1;
                    Some(item)
                } else {
                    None
                }
            }
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::FormItems;

//     impl<'f> From<&'f [(&'f str, &'f str, &'f str)]> for FormItems<'f> {
//         #[inline(always)]
//         fn from(triples: &'f [(&'f str, &'f str, &'f str)]) -> FormItems<'f> {
//             // Safe because RawStr(str) is repr(transparent).
//             let triples = unsafe { std::mem::transmute(triples) };
//             FormItems::Cooked { triples, next_index: 0 }
//         }
//     }

//     macro_rules! check_form {
//         (@bad $string:expr) => (check_form($string, None));
//         ($string:expr, $expected:expr) => (check_form(&$string[..], Some($expected)));
//     }

//     fn check_form<'a, T>(items: T, expected: Option<&[(&str, &str, &str)]>)
//         where T: Into<FormItems<'a>> + std::fmt::Debug
//     {
//         let string = format!("{:?}", items);
//         let mut items = items.into();
//         let results: Vec<_> = items.by_ref().map(|item| item.explode()).collect();
//         if let Some(expected) = expected {
//             assert_eq!(expected.len(), results.len(),
//                 "expected {:?}, got {:?} for {:?}", expected, results, string);

//             for i in 0..results.len() {
//                 let (expected_raw, expected_key, expected_val) = expected[i];
//                 let (actual_raw, actual_key, actual_val) = results[i];

//                 assert!(actual_raw == expected_raw,
//                         "raw [{}] mismatch for {}: expected {}, got {}",
//                         i, string, expected_raw, actual_raw);

//                 assert!(actual_key == expected_key,
//                         "key [{}] mismatch for {}: expected {}, got {}",
//                         i, string, expected_key, actual_key);

//                 assert!(actual_val == expected_val,
//                         "val [{}] mismatch for {}: expected {}, got {}",
//                         i, string, expected_val, actual_val);
//             }
//         } else {
//             assert!(!items.exhaust(), "{} unexpectedly parsed successfully", string);
//         }
//     }

//     #[test]
//     fn test_cooked_items() {
//         check_form!(
//             &[("username=user", "username", "user"), ("password=pass", "password", "pass")],
//             &[("username=user", "username", "user"), ("password=pass", "password", "pass")]
//         );

//         let empty: &[(&str, &str, &str)] = &[];
//         check_form!(empty, &[]);

//         check_form!(&[("a=b", "a", "b")], &[("a=b", "a", "b")]);

//         check_form!(
//             &[("user=x", "user", "x"), ("pass=word", "pass", "word"),
//                 ("x=z", "x", "z"), ("d=", "d", ""), ("e=", "e", "")],

//             &[("user=x", "user", "x"), ("pass=word", "pass", "word"),
//                 ("x=z", "x", "z"), ("d=", "d", ""), ("e=", "e", "")]
//         );
//     }

//     // #[test]
//     // fn test_form_string() {
//     //     check_form!("username=user&password=pass",
//     //                 &[("username", "user"), ("password", "pass")]);

//     //     check_form!("user=user&user=pass", &[("user", "user"), ("user", "pass")]);
//     //     check_form!("user=&password=pass", &[("user", ""), ("password", "pass")]);
//     //     check_form!("user&password=pass", &[("user", ""), ("password", "pass")]);
//     //     check_form!("foo&bar", &[("foo", ""), ("bar", "")]);

//     //     check_form!("a=b", &[("a", "b")]);
//     //     check_form!("value=Hello+World", &[("value", "Hello+World")]);

//     //     check_form!("user=", &[("user", "")]);
//     //     check_form!("user=&", &[("user", "")]);
//     //     check_form!("a=b&a=", &[("a", "b"), ("a", "")]);
//     //     check_form!("user=&password", &[("user", ""), ("password", "")]);
//     //     check_form!("a=b&a", &[("a", "b"), ("a", "")]);

//     //     check_form!("user=x&&", &[("user", "x")]);
//     //     check_form!("user=x&&&&pass=word", &[("user", "x"), ("pass", "word")]);
//     //     check_form!("user=x&&&&pass=word&&&x=z&d&&&e",
//     //                 &[("user", "x"), ("pass", "word"), ("x", "z"), ("d", ""), ("e", "")]);

//     //     check_form!("=&a=b&&=", &[("a", "b")]);
//     //     check_form!("=b", &[("", "b")]);
//     //     check_form!("=b&=c", &[("", "b"), ("", "c")]);

//     //     check_form!("=", &[]);
//     //     check_form!("&=&", &[]);
//     //     check_form!("&", &[]);
//     //     check_form!("=&=", &[]);

//     //     check_form!(@bad "=b&==");
//     //     check_form!(@bad "==");
//     //     check_form!(@bad "=k=");
//     //     check_form!(@bad "=abc=");
//     //     check_form!(@bad "=abc=cd");
//     // }
// }
