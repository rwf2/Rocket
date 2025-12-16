macro_rules! declare_span_macro {
    ($name:ident $level:ident) => (
        declare_span_macro!([$] $name $level);
    );

    ([$d:tt] $name:ident $level:ident) => (
        #[doc(hidden)]
        #[macro_export]
        macro_rules! $name {
            (@[$d ($t:tt)+] => $in_scope:expr) => ({
                $crate::tracing::span!($crate::tracing::Level::$level, $d ($t)+)
                    .in_scope(|| $in_scope);
            });

            (@[$d ($t:tt)+] $token:tt $d ($rest:tt)*) => ({
                $crate::trace::$name!(@[$d ($t)+ $token] $d ($rest)*);
            });

            // base case
            ($t:tt $d ($rest:tt)*) => ({
                $crate::trace::$name!(@[$t] $d ($rest)*);
            });
        }

        #[doc(hidden)]
        pub use $name as $name;
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! event {
    ($level:expr, $($args:tt)*) => {{
        match $level {
            $crate::tracing::Level::ERROR => event!(@$crate::tracing::Level::ERROR, $($args)*),
            $crate::tracing::Level::WARN => event!(@$crate::tracing::Level::WARN, $($args)*),
            $crate::tracing::Level::INFO => event!(@$crate::tracing::Level::INFO, $($args)*),
            $crate::tracing::Level::DEBUG => event!(@$crate::tracing::Level::DEBUG, $($args)*),
            $crate::tracing::Level::TRACE => event!(@$crate::tracing::Level::TRACE, $($args)*),
        }
    }};

    (@$level:expr, $n:expr, $($args:tt)*) => {{
        $crate::tracing::event!(name: $n, target: concat!("rocket::", $n), $level, $($args)*);
    }};
}

// Re-exports the macro at $path with the name $name. The point is to allow
// a `#[macro_use] extern crate rocket` to also automatically import the
// relevant tracing macros.
macro_rules! reexport {
    ($path:ident::$name:ident) => (
        reexport!([$] $path::$name);
    );

    ([ $d:tt ] $path:ident::$name:ident) => {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! $name {
            ($d ($f:tt)*) => {
                $crate::$path::$name!($d ($f)*)
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! span {
    ($level:expr, $($args:tt)*) => {{
        match $level {
            $crate::tracing::Level::ERROR =>
                $crate::tracing::span!($crate::tracing::Level::ERROR, $($args)*),
            $crate::tracing::Level::WARN =>
                $crate::tracing::span!($crate::tracing::Level::WARN, $($args)*),
            $crate::tracing::Level::INFO =>
                $crate::tracing::span!($crate::tracing::Level::INFO, $($args)*),
            $crate::tracing::Level::DEBUG =>
                $crate::tracing::span!($crate::tracing::Level::DEBUG, $($args)*),
            $crate::tracing::Level::TRACE =>
                $crate::tracing::span!($crate::tracing::Level::TRACE, $($args)*),
        }
    }};
}

#[doc(inline)]
pub use span;

declare_span_macro!(span_error ERROR);
declare_span_macro!(span_warn WARN);
declare_span_macro!(span_info INFO);
declare_span_macro!(span_debug DEBUG);
declare_span_macro!(span_trace TRACE);

#[doc(inline)]
pub use event;

reexport!(tracing::error);
reexport!(tracing::warn);
reexport!(tracing::info);
reexport!(tracing::debug);
reexport!(tracing::trace);

#[doc(hidden)]
pub use tracing::debug;
#[doc(hidden)]
pub use tracing::error;
#[doc(hidden)]
pub use tracing::info;
#[doc(hidden)]
pub use tracing::trace;
#[doc(hidden)]
pub use tracing::warn;
