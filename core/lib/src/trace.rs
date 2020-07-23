use crate::logger::{LoggingLevel, COLORS_ENV};
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

use yansi::Paint;

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

pub fn logging_layer<S>() -> impl Layer<S>
where
    S: tracing::Subscriber,
    S: for<'span> LookupSpan<'span>,
{
    let field_format = format::debug_fn(|writer, field, value| {
        // We'll format the field name and value separated with a colon.
        let name = field.name();
        if name == "message" {
            write!(writer, "{:?}", Paint::default(value).bold())
        } else {
            write!(writer, "{}: {:?}", field, Paint::default(value).bold())
        }
    })
    .delimited(", ")
    .display_messages();
    tracing_subscriber::fmt::layer()
        .fmt_fields(field_format)
        .event_format(EventFormat { last_id: AtomicU64::new(0) })
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
                write!(writer, "{:?}", Paint::default(value).bold())
            } else {
                write!(writer, "{}: {:?}", field, Paint::default(value).bold())
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
                    Paint::default("-->").bold(),
                    file,
                    line
                ),
                (Some(file), None) => write!(
                    f,
                    "{}\n    {} {}",
                    self.f,
                    Paint::default("-->").bold(),
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
