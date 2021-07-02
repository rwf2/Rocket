use ref_cast::RefCast;

use crate::http::RawStr;

/// A file name in a [`TempFile`] or multipart [`DataField`].
///
/// A `Content-Disposition` header, either in a response or a multipart field,
/// can optionally specify a `filename` directive as identifying information for
/// the attached file. This type represents the value of that directive.
///
/// # Safety
///
/// There are no restrictions on the value of the directive. In particular, the
/// value can be wholly unsafe to use as a file name in common contexts. As
/// such, Rocket sanitizes the value into a version that _is_ safe to use as a
/// file name in common contexts; this sanitized version can be retrieved via
/// [`FileName::as_str()`] and is returned by [`TempFile::name()`].
///
/// You will likely want to prepend or append random or user-specific components
/// to the name to avoid collisions; UUIDs make for a good "random" data. You
/// may also prefer to avoid the value in the directive entirely by using a
/// safe, application-generated name instead.
///
/// [`TempFile::name()`]: crate::fs::TempFile::name
/// [`DataField`]: crate::form::DataField
/// [`TempFile`]: crate::fs::TempFile
#[repr(transparent)]
#[derive(RefCast, Debug)]
pub struct FileName(str);

impl FileName {
    /// Wraps a string as a `FileName`. This is cost-free.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::FileName;
    ///
    /// let name = FileName::new("some-file.txt");
    /// assert_eq!(name.as_str(), Some("some-file"));
    ///
    /// let name = FileName::new("some-file.txt");
    /// assert_eq!(name.dangerous_unsafe_unsanitized_raw(), "some-file.txt");
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &FileName {
        FileName::ref_cast(string.as_ref())
    }

    /// The sanitized file name, stripped of any file extension and special
    /// characters, safe for use as a file name.
    ///
    /// # Sanitization
    ///
    /// A "sanitized" file name is a non-empty string, stripped of its file
    /// extension, which is not a platform-specific reserved name and does not
    /// contain any platform-specific special characters.
    ///
    /// On Unix, these are the characters `'.', '/', '\\', '<', '>', '|', ':',
    /// '(', ')', '&', ';', '#', '?', '*'`.
    ///
    /// On Windows (and non-Unix OSs), these are the characters `'.', '<', '>',
    /// ':', '"', '/', '\', '|', '?', '*', ',', ';', '=', '(', ')', '&', '#'`,
    /// and the reserved names `"CON", "PRN", "AUX", "NUL", "COM1", "COM2",
    /// "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    /// "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"`.
    ///
    /// Additionally, all control characters are considered "special".
    ///
    /// An attempt is made to transform the raw file name into a sanitized
    /// version by identifying a valid substring of the raw file name that meets
    /// this criteria. If none is found, `None` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::FileName;
    ///
    /// let name = FileName::new("some-file.txt");
    /// assert_eq!(name.as_str(), Some("some-file"));
    ///
    /// let name = FileName::new("some-file.txt.zip");
    /// assert_eq!(name.as_str(), Some("some-file"));
    ///
    /// let name = FileName::new("../../../../etc/shadow");
    /// assert_eq!(name.as_str(), Some("shadow"));
    ///
    /// let name = FileName::new("/etc/.shadow");
    /// assert_eq!(name.as_str(), Some("shadow"));
    ///
    /// let name = FileName::new("/a/b/some/file.txt.zip");
    /// assert_eq!(name.as_str(), Some("file"));
    ///
    /// let name = FileName::new("/a/b/some/.file.txt.zip");
    /// assert_eq!(name.as_str(), Some("file"));
    ///
    /// let name = FileName::new("/a/b/some/.*file.txt.zip");
    /// assert_eq!(name.as_str(), Some("file"));
    ///
    /// let name = FileName::new("a/\\b/some/.*file<.txt.zip");
    /// assert_eq!(name.as_str(), Some("file"));
    ///
    /// let name = FileName::new(">>>.foo.txt");
    /// assert_eq!(name.as_str(), Some("foo"));
    ///
    /// let name = FileName::new("b:c");
    /// #[cfg(unix)] assert_eq!(name.as_str(), Some("b"));
    /// #[cfg(not(unix))] assert_eq!(name.as_str(), Some("c"));
    ///
    /// let name = FileName::new("//./.<>");
    /// assert_eq!(name.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        #[cfg(not(unix))]
        let (bad_char, bad_name) = {
            static BAD_CHARS: &[char] = &[
                // Microsoft says these are invalid.
                '.', '<', '>', ':', '"', '/', '\\', '|', '?', '*',

                // `cmd.exe` treats these specially.
                ',', ';', '=',

                // These are treated specially by unix-like shells.
                '(', ')', '&', '#',
            ];

            // Microsoft says these are reserved.
            static BAD_NAMES: &[&str] = &[
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4",
                "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
                "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
            ];

            let bad_char = |c| BAD_CHARS.contains(&c) || c.is_control();
            let bad_name = |n| BAD_NAMES.contains(&n);
            (bad_char, bad_name)
        };

        #[cfg(unix)]
        let (bad_char, bad_name) = {
            static BAD_CHARS: &[char] = &[
                // These have special meaning in a file name.
                '.', '/', '\\',

                // These are treated specially by shells.
                '<', '>', '|', ':', '(', ')', '&', ';', '#', '?', '*',
            ];

            let bad_char = |c| BAD_CHARS.contains(&c) || c.is_control();
            let bad_name = |_| false;
            (bad_char, bad_name)
        };

        // Get the file name as a `str` without any extension(s).
        let file_name = std::path::Path::new(&self.0)
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split(bad_char).find(|s| !s.is_empty()))?;

        // At this point, `file_name` can't contain `bad_chars` because of
        // `.split()`, but it can be empty or reserved.
        if file_name.is_empty() || bad_name(file_name) {
            return None;
        }

        Some(file_name)
    }

    /// Returns `true` if the _complete_ raw file name is safe.
    ///
    /// Note that `.as_str()` returns a safe _subset_ of the raw file name, if
    /// there is one. If this method returns `true`, then that subset is the
    /// complete raw file name.
    ///
    /// This method should be use sparingly. In particular, there is no
    /// advantage to calling `is_safe()` prior to calling `as_str()`; simply
    /// call `as_str()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::FileName;
    ///
    /// let name = FileName::new("some-file.txt");
    /// assert_eq!(name.as_str(), Some("some-file"));
    /// assert!(!name.is_safe());
    ///
    /// let name = FileName::new("some-file");
    /// assert_eq!(name.as_str(), Some("some-file"));
    /// assert!(name.is_safe());
    /// ```
    pub fn is_safe(&self) -> bool {
        self.as_str().map_or(false, |s| s == &self.0)
    }

    /// The raw, unsanitized, potentially unsafe file name. Prefer to use
    /// [`FileName::as_str()`], always.
    ///
    /// # ⚠️ DANGER ⚠️
    ///
    /// This method returns the file name exactly as it was specified by the
    /// client. You should **_not_** use this name _unless_ you require the
    /// originally specified `filename` _and_ it is known not to contain
    /// special, potentially dangerous characters, _and_:
    ///
    ///   1. All clients are known to be trusted, perhaps because the server
    ///      only runs locally, serving known, local requests, or...
    ///
    ///   2. You will not use the file name to store a file on disk or any
    ///      context that expects a file name _and_ you will not use the
    ///      extension to determine how to handle/parse the data, or...
    ///
    ///   3. You will expertly process the raw name into a sanitized version for
    ///      use in specific contexts.
    ///
    /// If not all of these cases apply, use [`FileName::as_str()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::FileName;
    ///
    /// let name = FileName::new("some-file.txt");
    /// assert_eq!(name.dangerous_unsafe_unsanitized_raw(), "some-file.txt");
    ///
    /// let name = FileName::new("../../../../etc/shadow");
    /// assert_eq!(name.dangerous_unsafe_unsanitized_raw(), "../../../../etc/shadow");
    ///
    /// let name = FileName::new("../../.ssh/id_rsa");
    /// assert_eq!(name.dangerous_unsafe_unsanitized_raw(), "../../.ssh/id_rsa");
    ///
    /// let name = FileName::new("/Rocket.toml");
    /// assert_eq!(name.dangerous_unsafe_unsanitized_raw(), "/Rocket.toml");
    /// ```
    pub fn dangerous_unsafe_unsanitized_raw(&self) -> &RawStr {
        self.0.into()
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for &'a FileName {
    #[inline]
    fn from(string: &'a S) -> Self {
        FileName::new(string)
    }
}
