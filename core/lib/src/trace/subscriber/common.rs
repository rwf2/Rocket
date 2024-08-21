use std::fmt;
use std::cell::Cell;

use tracing::field::Field;
use tracing::{Level, Metadata};
use tracing_subscriber::filter;
use tracing_subscriber::field::RecordFields;

use thread_local::ThreadLocal;
use yansi::{Condition, Paint, Style};

use crate::config::CliColors;
use crate::trace::subscriber::RecordDisplay;
use crate::util::Formatter;

mod private {
    pub trait FmtKind: Send + Sync + 'static { }

    impl FmtKind for crate::trace::subscriber::Pretty { }
    impl FmtKind for crate::trace::subscriber::Compact { }
}

#[derive(Default)]
pub struct RocketFmt<K: private::FmtKind> {
    state: ThreadLocal<Cell<K>>,
    pub(crate) level: Option<Level>,
    pub(crate) filter: filter::Targets,
    pub(crate) style: Style,
}

impl<K: private::FmtKind + Default + Copy> RocketFmt<K> {
    pub(crate) fn state(&self) -> K {
        self.state.get_or_default().get()
    }

    pub(crate) fn update_state<F: FnOnce(&mut K)>(&self, update: F) {
        let cell = self.state.get_or_default();
        let mut old = cell.get();
        update(&mut old);
        cell.set(old);
    }
}

impl<K: private::FmtKind> RocketFmt<K> {
    pub fn new(workers: usize, cli_colors: CliColors, level: Option<Level>) -> Self {
        Self {
            state: ThreadLocal::with_capacity(workers),
            level,
            filter: filter::Targets::new()
                .with_default(level)
                .with_target("rustls", level.filter(|&l| l == Level::TRACE))
                .with_target("hyper", level.filter(|&l| l == Level::TRACE)),
            style: match cli_colors {
                CliColors::Always => Style::new().whenever(Condition::ALWAYS),
                CliColors::Auto => Style::new().whenever(Condition::DEFAULT),
                CliColors::Never => Style::new().whenever(Condition::NEVER),
            }
        }
    }

    pub fn style(&self, metadata: &Metadata<'_>) -> Style {
        match *metadata.level() {
            Level::ERROR => self.style.red(),
            Level::WARN => self.style.yellow(),
            Level::INFO => self.style.blue(),
            Level::DEBUG => self.style.green(),
            Level::TRACE => self.style.magenta(),
        }
    }

    pub(crate) fn has_message(&self, meta: &Metadata<'_>) -> bool {
        meta.fields().field("message").is_some()
    }

    pub(crate) fn has_data_fields(&self, meta: &Metadata<'_>) -> bool {
        meta.fields().iter().any(|f| f.name() != "message")
    }

    pub(crate) fn message<'a, F: RecordFields + 'a>(&self,
        init_prefix: &'a dyn fmt::Display,
        cont_prefix: &'a dyn fmt::Display,
        meta: &'a Metadata<'_>,
        data: F
    ) -> impl fmt::Display + 'a {
        let style = self.style(meta);
        Formatter(move |f| {
            let fields = meta.fields();
            let message = fields.field("message");
            if let Some(message_field) = &message {
                data.record_display(|field: &Field, value: &dyn fmt::Display| {
                    if field != message_field {
                        return;
                    }

                    for (i, line) in value.to_string().lines().enumerate() {
                        let line = line.paint(style);
                        if i == 0 {
                            let _ = writeln!(f, "{init_prefix}{line}");
                        } else {
                            let _ = writeln!(f, "{cont_prefix}{line}");
                        }
                    }
                });
            }

            Ok(())
        })
    }

    pub(crate) fn compact_fields<'a, F: RecordFields + 'a>(
        &self,
        meta: &'a Metadata<'_>,
        data: F
    ) -> impl fmt::Display + 'a {
        let key_style = self.style(meta).bold();
        let val_style = self.style(meta).primary();

        Formatter(move |f| {
            let mut printed = false;
            data.record_display(|field: &Field, val: &dyn fmt::Display| {
                let key = field.name();
                if key != "message" {
                    if printed { let _ = write!(f, " "); }
                    let _ = write!(f, "{}: {}", key.paint(key_style), val.paint(val_style));
                    printed = true;
                }
            });

            Ok(())
        })
    }

    pub(crate) fn print<F: RecordFields>(
        &self,
        prefix: &dyn fmt::Display,
        cont_prefix: &dyn fmt::Display,
        m: &Metadata<'_>,
        data: F
    ) {
        if self.has_message(m) {
            let message = self.message(prefix, cont_prefix, m, &data);
            if self.has_data_fields(m) {
                println!("{message}{cont_prefix}{}", self.compact_fields(m, &data));
            } else {
                print!("{message}");
            }
        } else if self.has_data_fields(m) {
            println!("{prefix}{}", self.compact_fields(m, &data));
        }
    }
}
