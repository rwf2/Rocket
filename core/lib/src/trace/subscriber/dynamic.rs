use std::sync::OnceLock;

use tracing::{Dispatch, Event, Level, Metadata};
use tracing::subscriber::{Subscriber, Interest};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::registry::{Registry, LookupSpan};
use tracing_subscriber::layer::{Context, Layer, Layered, SubscriberExt};
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::{Config, CliColors};
use crate::trace::subscriber::{Compact, Pretty, RequestId, RequestIdLayer, RocketFmt};
use crate::trace::TraceFormat;

pub struct RocketDynFmt {
    inner: either::Either<RocketFmt<Compact>, RocketFmt<Pretty>>,
}

impl From<RocketFmt<Compact>> for RocketDynFmt {
    fn from(value: RocketFmt<Compact>) -> Self {
        RocketDynFmt { inner: either::Either::Left(value) }
    }
}

impl From<RocketFmt<Pretty>> for RocketDynFmt {
    fn from(value: RocketFmt<Pretty>) -> Self {
        RocketDynFmt { inner: either::Either::Right(value) }
    }
}

impl RocketDynFmt {
    pub fn init(config: Option<&Config>) {
        type Handle = reload::Handle<RocketDynFmt, Layered<RequestIdLayer, Registry>>;

        static HANDLE: OnceLock<Handle> = OnceLock::new();

        // Do nothing if there's no config and we've already initialized.
        if config.is_none() && HANDLE.get().is_some() {
            return;
        }

        let workers = config.map_or(num_cpus::get(), |c| c.workers);
        let colors = config.map_or(CliColors::Auto, |c| c.cli_colors);
        let level = config.map_or(Some(Level::INFO), |c| c.log_level);
        let format = config.map_or(TraceFormat::Pretty, |c| c.log_format);

        let formatter = |format| match format {
            TraceFormat::Pretty => Self::from(RocketFmt::<Pretty>::new(workers, colors, level)),
            TraceFormat::Compact => Self::from(RocketFmt::<Compact>::new(workers, colors, level)),
        };

        let (layer, reload_handle) = reload::Layer::new(formatter(format));
        let result = tracing_subscriber::registry()
            .with(RequestId::layer())
            .with(layer)
            .try_init();

        if result.is_ok() {
            assert!(HANDLE.set(reload_handle).is_ok());
        } else if let Some(handle) = HANDLE.get() {
            assert!(handle.modify(|layer| *layer = formatter(format)).is_ok());
        }
    }

    pub fn reset(&mut self, cli_colors: CliColors, level: Option<Level>) {
        either::for_both!(&mut self.inner, f => f.reset(cli_colors, level))
    }
}

macro_rules! forward {
    ($T:ident => $(& $r:tt)? $method:ident ( $($p:ident : $t:ty),* ) $(-> $R:ty)?) => {
        #[inline(always)]
        fn $method(& $($r)? self $(, $p : $t)*) $(-> $R)? {
            match & $($r)* self.inner {
                either::Either::Left(layer) => Layer::<$T>::$method(layer, $($p),*),
                either::Either::Right(layer) => Layer::<$T>::$method(layer, $($p),*),
            }
        }
    };
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for RocketDynFmt {
    forward!(S => on_register_dispatch(subscriber: &Dispatch));
    forward!(S => &mut on_layer(subscriber: &mut S));
    forward!(S => register_callsite(metadata: &'static Metadata<'static>) -> Interest);
    forward!(S => enabled(metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool);
    forward!(S => on_new_span(attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>));
    forward!(S => on_record(_span: &Id, _values: &Record<'_>, _ctx: Context<'_, S>));
    forward!(S => on_follows_from(_span: &Id, _follows: &Id, _ctx: Context<'_, S>));
    forward!(S => event_enabled(_event: &Event<'_>, _ctx: Context<'_, S>) -> bool);
    forward!(S => on_event(_event: &Event<'_>, _ctx: Context<'_, S>));
    forward!(S => on_enter(_id: &Id, _ctx: Context<'_, S>));
    forward!(S => on_exit(_id: &Id, _ctx: Context<'_, S>));
    forward!(S => on_close(_id: Id, _ctx: Context<'_, S>));
    forward!(S => on_id_change(_old: &Id, _new: &Id, _ctx: Context<'_, S>));
}
