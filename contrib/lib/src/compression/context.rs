use std::ops::Deref;

crate struct Context {
    /// The root of the template directory.
    crate exceptions: Vec<String>,
}

impl Context {
    crate fn new() -> Context {
        Context {
            exceptions: vec![
                String::from("application/gzip"),
                String::from("application/brotli"),
                String::from("application/zip"),
                String::from("image/*"),
                String::from("application/wasm"),
                String::from("application/binary"),
            ],
        }
    }
    crate fn with_exceptions(exceps: Vec<String>) -> Context {
        Context { exceptions: exceps }
    }

    crate fn exceptions<'a>(&'a self) -> impl Deref<Target = Vec<String>> + 'a {
        &self.exceptions
    }
}

/// Wraps a Context. With `cfg(debug_assertions)` active, this structure
/// additionally provides a method to reload the context at runtime.
crate struct ContextManager(Context);

impl ContextManager {
    crate fn new(ctxt: Context) -> ContextManager {
        ContextManager(ctxt)
    }

    crate fn context<'a>(&'a self) -> impl Deref<Target = Context> + 'a {
        &self.0
    }
}
