use std::fmt;

use tracing::field::Field;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use yansi::{Paint, Painted};

use crate::trace::subscriber::{Data, RecordDisplay, RocketFmt};
use crate::util::Formatter;

#[derive(Debug, Default, Copy, Clone)]
pub struct Pretty {
    depth: u32,
}

impl RocketFmt<Pretty> {
    fn indent(&self) -> &'static str {
        static INDENT: &[&str] = &["", "   ", "      "];
        INDENT
            .get(self.state().depth as usize)
            .copied()
            .unwrap_or("         ")
    }

    fn marker(&self) -> &'static str {
        static MARKER: &[&str] = &["", ">> ", ":: "];
        MARKER
            .get(self.state().depth as usize)
            .copied()
            .unwrap_or("-- ")
    }

    fn emoji(&self, _emoji: &'static str) -> Painted<&'static str> {
        #[cfg(windows)]
        {
            "".paint(self.style).mask()
        }
        #[cfg(not(windows))]
        {
            _emoji.paint(self.style).mask()
        }
    }

    fn prefix<'a>(&self, meta: &'a Metadata<'_>) -> impl fmt::Display + 'a {
        let (i, m, s) = (self.indent(), self.marker(), self.style(meta));
        Formatter(move |f| match *meta.level() {
            Level::WARN => write!(f, "{i}{m}{} ", "warning:".paint(s).bold()),
            Level::ERROR => write!(f, "{i}{m}{} ", "error:".paint(s).bold()),
            Level::INFO => write!(f, "{i}{m}"),
            level => write!(f, "{i}{m}[{} {}] ", level.paint(s).bold(), meta.target()),
        })
    }

    fn print_pretty<F: RecordFields>(&self, m: &Metadata<'_>, data: F) {
        let prefix = self.prefix(m);
        let cont_prefix = Formatter(|f| {
            let style = self.style(m);
            write!(f, "{}{} ", self.indent(), "++".paint(style).dim())
        });

        self.print(&prefix, &cont_prefix, m, data);
    }

    fn print_fields<F>(&self, metadata: &Metadata<'_>, fields: F)
    where
        F: RecordFields,
    {
        let style = self.style(metadata);
        let prefix = self.prefix(metadata);
        fields.record_display(|key: &Field, value: &dyn fmt::Display| {
            if key.name() != "message" {
                println!(
                    "{prefix}{}: {}",
                    key.paint(style),
                    value.paint(style).primary()
                );
            }
        })
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for RocketFmt<Pretty> {
    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, S>) -> bool {
        self.filter
            .would_enable(metadata.target(), metadata.level())
    }

    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let (meta, data) = (event.metadata(), Data::new(event));
        let style = self.style(meta);
        match meta.name() {
            "config" => self.print_fields(meta, event),
            "liftoff" => {
                let prefix = self.prefix(meta);
                println!(
                    "{prefix}{}{} {}",
                    self.emoji("🚀 "),
                    "Rocket has launched on".paint(style).primary().bold(),
                    &data["endpoint"].paint(style).primary().bold().underline()
                );
            }
            "route" => println!(
                "{}",
                Formatter(|f| {
                    write!(
                        f,
                        "{}{}{}: ",
                        self.indent(),
                        self.marker(),
                        "route".paint(style)
                    )?;

                    let (base, mut relative) = (&data["uri.base"], &data["uri.unmounted"]);
                    if base.ends_with('/') && relative.starts_with('/') {
                        relative = &relative[1..];
                    }

                    write!(
                        f,
                        "{:>3} {} {}{}",
                        &data["rank"].paint(style.bright().dim()),
                        &data["method"].paint(style.bold()),
                        base.paint(style.primary().underline()),
                        relative.paint(style.primary()),
                    )?;

                    if let Some(name) = data.get("name") {
                        write!(f, " ({}", name.paint(style.bold().bright()))?;

                        if let Some(location) = data.get("location") {
                            write!(f, " {}", location.paint(style.dim()))?;
                        }

                        write!(f, ")")?;
                    }

                    Ok(())
                })
            ),
            "catcher" => println!(
                "{}",
                Formatter(|f| {
                    write!(
                        f,
                        "{}{}{}: ",
                        self.indent(),
                        self.marker(),
                        "catcher".paint(style)
                    )?;

                    match data.get("code") {
                        Some(code) => write!(f, "{} ", code.paint(style.bold()))?,
                        None => write!(f, "{} ", "default".paint(style.bold()))?,
                    }

                    write!(f, "{}", &data["uri.base"].paint(style.primary()))?;
                    if let Some(name) = data.get("name") {
                        write!(f, " ({}", name.paint(style.bold().bright()))?;

                        if let Some(location) = data.get("location") {
                            write!(f, " {}", location.paint(style.dim()))?;
                        }

                        write!(f, ")")?;
                    }

                    Ok(())
                })
            ),
            "header" => println!(
                "{}{}{}: {}: {}",
                self.indent(),
                self.marker(),
                "header".paint(style),
                &data["name"].paint(style.bold()),
                &data["value"].paint(style.primary()),
            ),
            "fairing" => println!(
                "{}{}{}: {} {}",
                self.indent(),
                self.marker(),
                "fairing".paint(style),
                &data["name"].paint(style.bold()),
                &data["kind"].paint(style.primary().dim()),
            ),
            _ => self.print_pretty(meta, event),
        }
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctxt: Context<'_, S>) {
        let data = Data::new(attrs);
        let span = ctxt.span(id).expect("new_span: span does not exist");
        if &data["count"] != "0" {
            let name = span.name();
            let icon = match name {
                "config" => "🔧 ",
                "routes" => "📬 ",
                "catchers" => "🚧 ",
                "fairings" => "📦 ",
                "shield" => "🛡️ ",
                "templating" => "📐 ",
                "request" => "● ",
                _ => "",
            };

            let meta = span.metadata();
            let style = self.style(meta);
            let emoji = self.emoji(icon);
            let name = name.paint(style).bold();

            let fields = self.compact_fields(meta, attrs);
            let prefix = self.prefix(meta);
            let fieldless_prefix = Formatter(|f| write!(f, "{prefix}{emoji}{name} "));
            let field_prefix = Formatter(|f| write!(f, "{prefix}{emoji}{name} ({fields}) "));

            if self.has_message(meta) && self.has_data_fields(meta) {
                print!(
                    "{}",
                    self.message(&field_prefix, &fieldless_prefix, meta, attrs)
                );
            } else if self.has_message(meta) {
                print!(
                    "{}",
                    self.message(&fieldless_prefix, &fieldless_prefix, meta, attrs)
                );
            } else if self.has_data_fields(meta) {
                println!("{field_prefix}");
            } else {
                println!("{fieldless_prefix}");
            }
        }

        span.extensions_mut().replace(data);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctxt: Context<'_, S>) {
        let span = ctxt.span(id).expect("new_span: span does not exist");
        match span.extensions_mut().get_mut::<Data>() {
            Some(data) => values.record(data),
            None => span.extensions_mut().insert(Data::new(values)),
        }

        let meta = span.metadata();
        println!("{}{}", self.prefix(meta), self.compact_fields(meta, values));
    }

    fn on_enter(&self, _: &Id, _: Context<'_, S>) {
        self.update_state(|state| state.depth = state.depth.saturating_add(1));
    }

    fn on_exit(&self, _: &Id, _: Context<'_, S>) {
        self.update_state(|state| state.depth = state.depth.saturating_sub(1));
    }
}
