/// Form guard options.
///
/// See [`Form#leniency`](crate::form::Form#leniency) for details.
#[non_exhaustive] // TODO Remove when more than one option is available.
                  // This attribute was added to avoid Clippy's "needless_update" warning inside
                  // the function strict::init.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Options {
    /// Whether parsing should be strict (no extra parameters) or not.
    pub strict: bool,
}

#[allow(non_upper_case_globals, dead_code)]
impl Options {
    /// `Options` with `strict` set to `false`.
    pub const Lenient: Self = Options { strict: false };

    /// `Options` with `strict` set to `true`.
    pub const Strict: Self = Options { strict: true };
}
