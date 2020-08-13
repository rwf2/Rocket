//! Tracing, telemetry, and logging.
//!
//! Rocket provides support for application-level diagnostics using
//! the [`tracing`] crate. `tracing` provides a _facade layer_ for scoped,
//! structured, application-level diagnostics. This means that diagnostic data
//! from Rocket applications, from Rocket itself, and from any dependencies that
//! use the [`tracing`] or [`log`] crates, can be emitted in a machine-readable
//! format and consumed in a wide variety of ways, including structured logging,
//! distributed tracing, and performance profiling.
//!
//! This module re-exports core components of the `tracing` API for use in
//! Rocket applications, and provides Rocket-specific APIs for use with
//! `tracing`.
//!
//! # Using Tracing
//!
//! Tracing's data model is based around two core concepts: [_spans_][spans] and
//! [_events_][events]. A span represents a _period of time_, with a beginning and
//! an end, during which a program was executing in a particular context or
//! performing a unit of work. An event represents a _momentary_ occurance
//! within a trace, comparable to a single log line.
//!
//! Spans and events are recorded using macros, the basics of which are likely
//! familiar to users of the [`log`] crate. The [`trace!`], [`debug!`],
//! [`info!`], [`warn!`], and [`error!`] macros record an event at a priority
//! level ranging from extremely verbose diagnostic information (`trace!`) to
//! high-priority warnings (`error!`). For example:
//!
//! ```
//! use rocket::trace;
//!
//! trace::trace!("Apollo 13 presently at 177,861 nautical miles away.");
//! trace::debug!("Velocity now reading 3,263 feet per second.");
//! trace::info!("13, we'd like you to stir up your cryo tanks.");
//! trace::warn!("Houston, we've got a Main Bus B undervolt.");
//! trace::error!("Houston, we are venting something out into space!");
//! ```
//!
//! An event consists of one or more key-value pairs, called _fields_, and/or a
//! textual, human-readable _message_. For example, this will record an event
//! at the `info` level, with two fields, named `answer` and `question`:
//!
//! ```
//! # use rocket::trace;
//! trace::info!(answer = 42, question = "life, the universe, and everything");
//! ```
//! The [`tracing` documentation][macros] provides details on how these macros are used.
//!
//! Spans may be recorded in a few different ways. Like events, they have a
//! priority level, and may have one or more fields. In addition, all spans also
//! have a _name_. The easiest way to record a span is to add the
//! [`#[tracing::instrument]`][instrument] attribute to a function. For example:
//!
//!
//! ```
//! use rocket::trace;
//!
//! # #[derive(Debug)] struct Planet;
//! // Calling this function will enter a new span named `jump_to_hyperspace`.
//! #[trace::instrument]
//! async fn jump_to_hyperspace(destination: Planet) {
//!     // This event will be recorded *within* the `jump_to_hyperspace` span.
//!     tracing::debug!("preparing to jump to hyperspace...");
//!
//!    // ...
//! }
//! ```
//! This will automatically create a span with the same name as the instrumented
//! function, and all the arguments to that function recorded as fields.
//! Additional arguments to `#[instrument]` allow customizing the span further.
//! See the [`tracing` crate's documentation](instrument) for details.
//!
//! In addition, spans may be created manually using the [`trace_span!`],
//! [`debug_span!`], [`info_span!`], [`warn_span!`], and [`error_span!`] macros.
//! Again, the [`tracing` documentation][macros] provides further details on how
//! to use these macros.
//!
//! # Customization
//!
//! Spans and events are recorded by a `tracing` component called a
//! [`Subscriber`], which implements a particular way of collecting and
//! recording trace data. By default, Rocket provides its own `Subscriber`
//! implementation, which logs events to the console. This `Subscriber` will be
//! installed when [`rocket::ignite`] is called.
//!
//! To override the default `Subscriber` with another implementation, simply
//! [set it as the default][default] prior to calling `rocket::ignite`. For
//! example:
//! ```
//! # type MySubscriber = tracing_subscriber::registry::Registry;
//! #[rocket::launch]
//! fn rocket() -> rocket::Rocket {
//!     let subscriber = MySubscriber::default();
//!     tracing::subscriber::set_global_default(subscriber)
//!         .expect("the global default subscriber should not have been set");
//!
//!     rocket::ignite()
//!         // ...
//! }
//! ```
//!
//! Since `tracing` data is structured and machine-readable, it may be collected
//! in a variety of ways. The `tracing` community provides [several crates] for
//! logging in several commonly-used formats, emitting distributed tracing data
//! to collectors like [OpenTelemetry] and [honeycomb.io], and for
//! [multiple][timing] [forms][flame] of performance profiling.
//!
//! The [`tracing-subscriber`] crate provides an abstraction for building
//! a `Subscriber` by composing multiple [`Layer`]s which implement different
//! ways of collecting traces. This allows applications to record the same trace
//! data in multiple ways.
//!
//! In addition to providing a default subscriber out of the box, Rocket also
//! exposes its default logging and filtering behavior as `Layer`s. This means
//! that users who would like to combine the default logging layer with layers
//! from other crates may do so. For example:
//!
//! ```rust
//! # use tracing_subscriber::Layer;
//! # #[derive(Default)] struct SomeLayer;
//! # impl<S: tracing::Subscriber + 'static> Layer<S> for SomeLayer {}
//! # #[derive(Default)] struct SomeOtherLayer;
//! # impl<S: tracing::Subscriber + 'static> Layer<S> for SomeOtherLayer {}
//! #[rocket::launch]
//! fn rocket() -> rocket::Rocket {
//!     use rocket::trace::prelude::*;
//!
//!     let config = rocket::Config::read()
//!         .expect("failed to read config!");
//!
//!     // Configure our trace subscriber...
//!     tracing_subscriber::registry()
//!         // Add Rocket's default log formatter.
//!         .with(rocket::trace::logging_layer())
//!         // Add a custom layer...
//!         .with(SomeLayer::default())
//!         // ...and another custom layer.
//!         .with(SomeOtherLayer::default())
//!         // Filter what traces are enabled based on the Rocket config.
//!         .with(rocket::filter_layer(config.log_level))
//!         // Set our subscriber as the default.
//!         .init();
//!
//!     rocket::custom(config)
//!         // ...
//! }
//! ```
//!
//! [`tracing`]: https://docs.rs/tracing
//! [`log`]: https://docs.rs/log/
//! [spans]: https://docs.rs/tracing/latest/tracing/#spans
//! [events]: https://docs.rs/tracing/latest/tracing/#events
//! [`span!`]: https://docs.rs/tracing/latest/tracing/macro.span.html
//! [`event!`]: https://docs.rs/tracing/latest/tracing/macro.event.html
//! [`trace!`]: https://docs.rs/tracing/latest/tracing/macro.trace.html
//! [`debug!`]: https://docs.rs/tracing/latest/tracing/macro.debug.html
//! [`info!`]: https://docs.rs/tracing/latest/tracing/macro.info.html
//! [`warn!`]: https://docs.rs/tracing/latest/tracing/macro.warn.html
//! [`error!`]: https://docs.rs/tracing/latest/tracing/macro.error.html
//! [macros]: https://docs.rs/tracing/latest/tracing/index.html#using-the-macros
//! [instrument]: https://docs.rs/tracing/latest/tracing/attr.instrument.html
//! [`trace_span!`]: https://docs.rs/tracing/latest/tracing/macro.trace_span.html
//! [`debug_span!`]: https://docs.rs/tracing/latest/tracing/macro.debug_span.html
//! [`info_span!`]: https://docs.rs/tracing/latest/tracing/macro.info_span.html
//! [`warn_span!`]: https://docs.rs/tracing/latest/tracing/macro.warn_span.html
//! [`error_span!`]: https://docs.rs/tracing/latest/tracing/macro.error_span.html
//! [`rocket::ignite`]: crate::ignite
//! [default]: https://docs.rs/tracing/latest/tracing/#in-executables
//! [`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
//! [several crates]: https://github.com/tokio-rs/tracing#related-crates
//! [`tracing-subscriber`]: https://docs.rs/tracing-subscriber/
//! [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//! [OpenTelemetry]: https://crates.io/crates/tracing-opentelemetry
//! [honeycomb.io]: https://crates.io/crates/tracing-honeycomb
//! [timing]: https://crates.io/crates/tracing-timing
//! [flame]: https://crates.io/crates/tracing-flame
use tracing_subscriber::{
    field,
    fmt::{
        format::{self, FormatEvent, FormatFields},
        FmtContext, FormattedFields,
    },
    layer::Layer,
    prelude::*,
    registry::LookupSpan,
};

use std::env;
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering::{Acquire, Release}};
use std::str::FromStr;

use yansi::Paint;

pub use tracing::{
    trace, debug, info, warn, error, trace_span, debug_span, warn_span,
    error_span, instrument,
};

/// A prelude for working with `tracing` in Rocket applications.
pub mod prelude {
    pub use tracing_subscriber::prelude::*;
    pub use tracing_futures::Instrument as _;
}

/// Defines the different levels for log messages.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LoggingLevel {
    /// Only shows errors, warnings, and launch information.
    Critical,
    /// Shows everything except debug and trace information.
    Normal,
    /// Shows everything.
    Debug,
    /// Shows nothing.
    Off,
}

impl FromStr for LoggingLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match s {
            "critical" => LoggingLevel::Critical,
            "normal" => LoggingLevel::Normal,
            "debug" => LoggingLevel::Debug,
            "off" => LoggingLevel::Off,
            _ => return Err("a log level (off, debug, normal, critical)"),
        };

        Ok(level)
    }
}

impl fmt::Display for LoggingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match *self {
            LoggingLevel::Critical => "critical",
            LoggingLevel::Normal => "normal",
            LoggingLevel::Debug => "debug",
            LoggingLevel::Off => "off",
        };

        write!(f, "{}", string)
    }
}

/// Returns a Rocket filtering [`Layer`] based on the provided logging level.
///
/// The returned [`Layer`] can be added to another `tracing` subscriber to
/// configure it to filter spans and events based on the logging level
/// specified in the Rocket config.
///
/// For example:
///
/// ```
/// # type MySubscriber = tracing_subscriber::registry::Registry;
/// #[rocket::launch]
/// fn rocket() -> rocket::Rocket {
///     use rocket::trace::prelude::*;
///
///     let config = rocket::Config::read()
///         .expect("failed to read config!");
///
///     // Use some `tracing` subscriber from another crate...
///     MySubscriber::default()
///         // ...but filter spans and events based on the Rocket
///         // config file.
///         .with(rocket::trace::filter_layer(config.log_level))
///         .init();
///
///     rocket::custom(config)
///         // ...
/// }
/// ```
///
/// [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
pub fn filter_layer<S>(level: LoggingLevel) -> impl Layer<S>
where
    S: tracing::Subscriber,
{
    let filter_str = match level {
        LoggingLevel::Critical => "warn,rocket::launch=info,hyper=off,rustls=off",
        LoggingLevel::Normal => "info,hyper=off,rustls=off",
        LoggingLevel::Debug => "trace",
        LoggingLevel::Off => "off",
    };

    tracing_subscriber::filter::EnvFilter::try_new(filter_str)
        .expect("filter string must parse")
}

/// Returns a Rocket-style log formatting layer.
///
/// The returned layer can be added to a [`tracing-subscriber`
/// `Registry`][registry] to add Rocket-style log formatting in addition to
/// other [`Layer`s] providing different functionality.
///
/// For example:
///
/// ```
/// # type MySubscriber = tracing_subscriber::registry::Registry;
/// #[rocket::launch]
/// fn rocket() -> rocket::Rocket {
///     use rocket::trace::prelude::*;
///
///     let config = rocket::Config::read()
///         .expect("failed to read config!");
///
///     // Use some `tracing` subscriber from another crate...
///     MySubscriber::default()
///         // ...but filter spans and events based on the Rocket
///         // config file.
///         .with(rocket::trace::filter_layer(config.log_level))
///         .init();
///
///     rocket::custom(config)
///         // ...
/// }
/// ```
///
/// [`Layer`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
/// [`registry`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/registry/index.html
pub fn logging_layer<S>() -> impl Layer<S>
where
    S: tracing::Subscriber,
    S: for<'span> LookupSpan<'span>,
{
    let field_format = format::debug_fn(|writer, field, value| {
        // We'll format the field name and value separated with a colon.
        let name = field.name();
        if name == "message" {
            write!(writer, "{:?}", Paint::new(value).bold())
        } else {
            write!(writer, "{}: {:?}", field, Paint::new(value).bold())
        }
    })
    .delimited(", ")
    .display_messages();
    tracing_subscriber::fmt::layer()
        .fmt_fields(field_format)
        .event_format(EventFormat { last_id: AtomicU64::new(0) })
}

pub(crate) const COLORS_ENV: &str = "ROCKET_CLI_COLORS";

pub(crate) fn try_init(level: LoggingLevel) -> bool {
    if level == LoggingLevel::Off {
        return false;
    }

    if !atty::is(atty::Stream::Stdout)
        || (cfg!(windows) && !Paint::enable_windows_ascii())
        || env::var_os(COLORS_ENV)
            .map(|v| v == "0" || v == "off")
            .unwrap_or(false)
    {
        Paint::disable();
    }

    tracing::subscriber::set_global_default(tracing_subscriber::registry()
        .with(logging_layer())
        .with(filter_layer(level))
    )
        .is_ok()
}

pub(crate) trait PaintExt {
    fn emoji(item: &str) -> Paint<&str>;
}

impl PaintExt for Paint<&str> {
    /// Paint::masked(), but hidden on Windows due to broken output. See #1122.
    fn emoji(item: &str) -> Paint<&str> {
        if cfg!(windows) {
            Paint::masked("")
        } else {
            Paint::masked(item)
        }
    }
}


struct EventFormat {
    last_id: AtomicU64,
}

impl<S, N> FormatEvent<S, N> for EventFormat
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        cx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let mut seen = false;
        let id = if let Some(span) = cx.lookup_current() {
            let id = span.id();
            if id.into_u64() != self.last_id.load(Acquire) {
                cx.visit_spans(|span| {
                    if seen {
                        write!(writer, "    {} ", Paint::default("=>").bold())?;
                    }
                    let meta = span.metadata();
                    let exts = span.extensions();
                    if let Some(fields) = exts.get::<FormattedFields<N>>() {
                        // If the span has a human-readable message, print that
                        // instead of the span's name (so that we can get nice emojis).
                        if meta.fields().iter().any(|field| field.name() == "message") {
                            with_meta(writer, meta, &fields.fields)?;
                        } else {
                            with_meta(writer, meta, format_args!("{} {}", span.name(), &fields.fields))?;
                        }
                    } else {
                        with_meta(writer, span.metadata(), span.name())?;
                    }
                    seen = true;
                    Ok(())
                })?;
            } else {
                seen = true;
            }
            Some(id)
        } else {
            None
        };

        if seen {
            write!(writer, "    {} ", Paint::default("=>").bold())?;
        }

        // xxx(eliza): workaround
        let fmt = format::debug_fn(|writer, field, value| {
            // We'll format the field name and value separated with a colon.
            let name = field.name();
            if name == "message" {
                write!(writer, "{:?}", Paint::new(value).bold())
            } else {
                write!(writer, "{}: {:?}", field, Paint::new(value).bold())
            }
        })
        .delimited(", ")
        .display_messages();
        with_meta(
            writer,
            event.metadata(),
            &FmtVisitor {
                fmt: &fmt,
                records: event,
            },
        )?;
        if let Some(id) = id {
            self.last_id.store(id.into_u64(), Release);
        }
    Ok(())
    }
}

fn with_meta(
    writer: &mut dyn Write,
    meta: &tracing::Metadata<'_>,
    f: impl fmt::Display,
) -> fmt::Result {

    struct WithFile<'a, F> {
        meta: &'a tracing::Metadata<'a>,
        f: F,
    }

    impl<F: fmt::Display> fmt::Display for WithFile<'_, F> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match (self.meta.file(), self.meta.line()) {
                (Some(file), Some(line)) => write!(
                    f,
                    "{}\n    {} {}:{}",
                    self.f,
                    Paint::new("-->").bold(),
                    file,
                    line
                ),
                (Some(file), None) => write!(
                    f,
                    "{}\n    {} {}",
                    self.f,
                    Paint::new("-->").bold(),
                    file,
                ),
                _ => write!(f,  "{}", self.f),
            }
        }
    }


    match *meta.level() {
        tracing::Level::INFO => writeln!(writer, "{}", Paint::blue(f).wrap()),
        tracing::Level::ERROR => writeln!(
            writer,
            "{} {}",
            Paint::red("Error:").bold(),
            Paint::red(f).wrap()
        ),
        tracing::Level::WARN => writeln!(
            writer,
            "{} {}",
            Paint::yellow("Warning:").bold(),
            Paint::yellow(f).wrap()
        ),
        tracing::Level::TRACE => writeln!(writer, "{}", Paint::magenta(WithFile { meta, f }).wrap()),
        tracing::Level::DEBUG => writeln!(writer, "{}", Paint::blue(WithFile { meta, f }).wrap()),
    }
}

struct FmtVisitor<'a, F, R> {
    fmt: &'a F,
    records: R,
}

impl<F, R> fmt::Display for FmtVisitor<'_, F, R>
where
    F: for<'w> FormatFields<'w>,
    R: field::RecordFields,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt.format_fields(f, &self.records)
    }
}
