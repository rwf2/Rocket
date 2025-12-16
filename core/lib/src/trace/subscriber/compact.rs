use std::fmt;
use std::num::NonZeroU64;
use std::time::Instant;

use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use time::OffsetDateTime;
use yansi::{Paint, Painted};

use super::RecordDisplay;
use crate::http::{Status, StatusClass};
use crate::trace::subscriber::{Data, RocketFmt};
use crate::util::Formatter;

#[derive(Debug, Default, Copy, Clone)]
pub struct Compact {
    /// The `tracing::Span::Id` of the request we're in, if any.
    request: Option<NonZeroU64>,
}

#[derive(Debug)]
pub struct RequestData {
    start: Instant,
    fields: Data,
    item: Option<(String, String)>,
}

impl RequestData {
    pub fn new<T: RecordFields>(attrs: T) -> Self {
        Self {
            start: Instant::now(),
            fields: Data::new(attrs),
            item: None,
        }
    }
}

impl RocketFmt<Compact> {
    fn request_span_id(&self) -> Option<Id> {
        self.state().request.map(Id::from_non_zero_u64)
    }

    fn timestamp_for(&self, datetime: OffsetDateTime) -> impl fmt::Display {
        Formatter(move |f| {
            let (date, time) = (datetime.date(), datetime.time());
            let (year, month, day) = (date.year(), date.month() as u8, date.day());
            let (h, m, s, l) = (
                time.hour(),
                time.minute(),
                time.second(),
                time.millisecond(),
            );
            write!(
                f,
                "{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{l:03}Z"
            )
        })
    }

    fn in_debug(&self) -> bool {
        self.level.is_some_and(|l| l >= Level::DEBUG)
    }

    fn prefix<'a>(&self, meta: &'a Metadata<'_>) -> impl fmt::Display + 'a {
        let style = self.style(meta);
        let name = if meta.name().starts_with("event ") {
            meta.target()
        } else {
            meta.name()
        };

        let pad = self.level.map_or(0, |lvl| lvl.as_str().len());
        let timestamp = self.timestamp_for(OffsetDateTime::now_utc());
        Formatter(move |f| {
            write!(
                f,
                "{} {:>pad$} {} ",
                timestamp.paint(style).primary().dim(),
                meta.level().paint(style),
                name.paint(style).primary()
            )
        })
    }

    fn chevron(&self, meta: &Metadata<'_>) -> Painted<&'static str> {
        "›".paint(self.style(meta)).bold()
    }

    fn print_compact<F: RecordFields>(&self, m: &Metadata<'_>, data: F) {
        let style = self.style(m);
        let prefix = self.prefix(m);
        let chevron = self.chevron(m);
        let init_prefix = Formatter(|f| write!(f, "{prefix}{chevron} "));
        let cont_prefix = Formatter(|f| write!(f, "{prefix}{} ", "+".paint(style).dim()));
        self.print(&init_prefix, &cont_prefix, m, data);
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for RocketFmt<Compact> {
    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, S>) -> bool {
        self.filter
            .would_enable(metadata.target(), metadata.level())
            && (self.in_debug()
                || self.request_span_id().is_none()
                || metadata.name() == "request"
                || metadata.name() == "response")
    }

    fn on_event(&self, event: &Event<'_>, ctxt: Context<'_, S>) {
        if let Some(id) = self.request_span_id() {
            let name = event.metadata().name();
            if name == "response" {
                let req_span = ctxt.span(&id).expect("on_event: req does not exist");
                let mut exts = req_span.extensions_mut();
                let data = exts.get_mut::<RequestData>().unwrap();
                event.record(&mut data.fields);
            } else if name == "catcher" || name == "route" {
                let req_span = ctxt.span(&id).expect("on_event: req does not exist");
                let mut exts = req_span.extensions_mut();
                let data = exts.get_mut::<RequestData>().unwrap();
                data.item = event.find_map_display("name", |v| (name.into(), v.to_string()))
            }

            if !self.in_debug() {
                return;
            }
        }

        self.print_compact(event.metadata(), event);
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("new_span: span does not exist");

        if span.name() == "request" {
            let data = RequestData::new(attrs);
            span.extensions_mut().replace(data);

            if !self.in_debug() {
                return;
            }
        }

        if self.state().request.is_none() {
            self.print_compact(span.metadata(), attrs);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("record: span does not exist");
        if self.request_span_id().as_ref() == Some(id) {
            let mut extensions = span.extensions_mut();
            match extensions.get_mut::<RequestData>() {
                Some(data) => values.record(&mut data.fields),
                None => span.extensions_mut().insert(RequestData::new(values)),
            }
        }

        if self.in_debug() {
            println!(
                "{}{} {}",
                self.prefix(span.metadata()),
                self.chevron(span.metadata()),
                self.compact_fields(span.metadata(), values)
            );
        }
    }

    fn on_enter(&self, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("new_span: span does not exist");
        if span.name() == "request" {
            self.update_state(|state| state.request = Some(id.into_non_zero_u64()));
        }
    }

    fn on_exit(&self, id: &Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("new_span: span does not exist");
        if span.name() == "request" {
            self.update_state(|state| state.request = None);
        }
    }

    fn on_close(&self, id: Id, ctxt: Context<'_, S>) {
        let span = ctxt.span(&id).expect("new_span: span does not exist");
        if span.name() == "request" {
            let extensions = span.extensions();
            let data = extensions.get::<RequestData>().unwrap();

            let elapsed = data.start.elapsed();
            let datetime = OffsetDateTime::now_utc() - elapsed;
            let timestamp = self.timestamp_for(datetime);

            let s = self.style(span.metadata());
            let prefix = self.prefix(span.metadata());
            let chevron = self.chevron(span.metadata());
            let arrow = "→".paint(s.primary().bright());

            let status_class = data.fields["status"]
                .parse()
                .ok()
                .and_then(Status::from_code)
                .map(|status| status.class());

            let status_style = match status_class {
                Some(StatusClass::Informational) => s,
                Some(StatusClass::Success) => s.green(),
                Some(StatusClass::Redirection) => s.magenta(),
                Some(StatusClass::ClientError) => s.yellow(),
                Some(StatusClass::ServerError) => s.red(),
                Some(StatusClass::Unknown) => s.cyan(),
                None => s.primary(),
            };

            let autohandle = Formatter(|f| match data.fields.get("autohandled") {
                Some("true") => write!(f, " {} {}", "via".paint(s.dim()), "GET".paint(s)),
                _ => Ok(()),
            });

            let item = Formatter(|f| match &data.item {
                Some((kind, name)) => {
                    write!(f, "{} {} {arrow} ", kind.paint(s), name.paint(s.bold()),)
                }
                None => Ok(()),
            });

            println!(
                "{prefix}{chevron} ({} {}ms) {}{autohandle} {} {arrow} {item}{}",
                timestamp.paint(s).primary().dim(),
                elapsed.as_millis(),
                &data.fields["method"].paint(s),
                &data.fields["uri"],
                &data.fields["status"].paint(status_style),
            );
        }
    }
}
