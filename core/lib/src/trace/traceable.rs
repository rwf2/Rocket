use std::error::Error as StdError;

use crate::request::ConnectionMeta;
use crate::sentinel::Sentry;
use crate::util::Formatter;
use crate::{route, Catcher, Config, Error, Request, Response, Route};
use crate::error::ErrorKind;

use figment::Figment;
use rocket::http::Header;
use tracing::Level;

pub trait Trace {
    fn trace(&self, level: Level);

    #[inline(always)] fn trace_info(&self) { self.trace(Level::INFO) }
    #[inline(always)] fn trace_warn(&self) { self.trace(Level::WARN) }
    #[inline(always)] fn trace_error(&self) { self.trace(Level::ERROR) }
    #[inline(always)] fn trace_debug(&self) { self.trace(Level::DEBUG) }
    #[inline(always)] fn trace_trace(&self) { self.trace(Level::TRACE) }
}

pub trait TraceAll: Sized {
    fn trace_all(self, level: Level);

    #[inline(always)] fn trace_all_info(self) { self.trace_all(Level::INFO) }
    #[inline(always)] fn trace_all_warn(self) { self.trace_all(Level::WARN) }
    #[inline(always)] fn trace_all_error(self) { self.trace_all(Level::ERROR) }
    #[inline(always)] fn trace_all_debug(self) { self.trace_all(Level::DEBUG) }
    #[inline(always)] fn trace_all_trace(self) { self.trace_all(Level::TRACE) }
}

impl<T: Trace, I: IntoIterator<Item = T>> TraceAll for I {
    fn trace_all(self, level: Level) {
        self.into_iter().for_each(|i| i.trace(level))
    }
}

impl<T: Trace> Trace for &T {
    #[inline(always)]
    fn trace(&self, level: Level) {
        T::trace(self, level)
    }
}

impl Trace for Figment {
    fn trace(&self, level: Level) {
        for param in Config::PARAMETERS {
            if let Some(source) = self.find_metadata(param) {
                if param.contains("secret") {
                    continue;
                }

                event! { level, "figment",
                    param,
                    %source.name,
                    source.source = source.source.as_ref().map(display),
                    value = self.find_value(param).ok().map(debug),
                }
            }
        }

        // Check for now deprecated config values.
        for (key, replacement) in Config::DEPRECATED_KEYS {
            if let Some(source) = self.find_metadata(key) {
                event! { Level::WARN, "deprecated",
                    key,
                    replacement,
                    %source.name,
                    source.source = source.source.as_ref().map(display),
                    "config key `{key}` is deprecated and has no meaning"
                }
            }
        }
    }
}

impl Trace for Config {
    fn trace(&self, level: Level) {
        event! { level, "config",
            http2 = cfg!(feature = "http2"),
            log_level = self.log_level.map(|l| l.as_str()),
            log_format = ?self.log_format,
            cli_colors = %self.cli_colors,
            workers = self.workers,
            max_blocking = self.max_blocking,
            ident = %self.ident,
            ip_header = self.ip_header.as_ref().map(|s| s.as_str()),
            proxy_proto_header = self.proxy_proto_header.as_ref().map(|s| s.as_str()),
            limits = %Formatter(|f| f.debug_map()
                .entries(self.limits.limits.iter().map(|(k, v)| (k.as_str(), display(v))))
                .finish()),
            temp_dir = %self.temp_dir.relative().display(),
            keep_alive = (self.keep_alive != 0).then_some(self.keep_alive),
            shutdown.ctrlc = self.shutdown.ctrlc,
            shutdown.signals = %{
                #[cfg(not(unix))] {
                    "disabled (not unix)"
                }

                #[cfg(unix)] {
                    Formatter(|f| f.debug_set()
                        .entries(self.shutdown.signals.iter().map(|s| s.as_str()))
                        .finish())
                }
            },
                shutdown.grace = self.shutdown.grace,
                shutdown.mercy = self.shutdown.mercy,
                shutdown.force = self.shutdown.force,
        }

        #[cfg(feature = "secrets")] {
            if !self.secret_key.is_provided() {
                warn! {
                    name: "volatile_secret_key",
                    "secrets enabled without configuring a stable `secret_key`\n\
                    private/signed cookies will become unreadable after restarting\n\
                    disable the `secrets` feature or configure a `secret_key`\n\
                    this becomes a hard error in non-debug profiles",
                }
            }

            let secret_key_is_known = Config::KNOWN_SECRET_KEYS.iter().any(|&key_str| {
                let value = figment::value::Value::from(key_str);
                self.secret_key == value.deserialize().expect("known key is valid")
            });

            if secret_key_is_known {
                warn! {
                    name: "insecure_secret_key",
                    "The configured `secret_key` is exposed and insecure. \
                    The configured key is publicly published and thus insecure. \
                    Try generating a new key with `head -c64 /dev/urandom | base64`."
                }
            }
        }
    }
}

impl Trace for Route {
    fn trace(&self, level: Level) {
        event! { level, "route",
            name = self.name.as_ref().map(|n| &**n),
            rank = self.rank,
            method = %self.method,
            uri = %self.uri,
            uri.base = %self.uri.base(),
            uri.unmounted = %self.uri.unmounted(),
            format = self.format.as_ref().map(display),
            location = self.location.as_ref()
                .map(|(file, line, _)| Formatter(move |f| write!(f, "{file}:{line}")))
                .map(display),
        }

        event! { Level::DEBUG, "sentinels",
            route = self.name.as_ref().map(|n| &**n),
            sentinels = %Formatter(|f| {
                f.debug_set()
                    .entries(self.sentinels.iter().filter(|s| s.specialized).map(|s| s.type_name))
                    .finish()
            })
        }
    }
}

impl Trace for Catcher {
    fn trace(&self, level: Level) {
        event! { level, "catcher",
            name = self.name.as_ref().map(|n| &**n),
            code = %Formatter(|f| match self.code {
                Some(code) => write!(f, "{}", code),
                None => write!(f, "default"),
            }),
            rank = self.rank,
            uri.base = %self.base(),
            location = self.location.as_ref()
                .map(|(file, line, _)| Formatter(move |f| write!(f, "{file}:{line}")))
                .map(display),
        }
    }
}

impl Trace for &dyn crate::fairing::Fairing {
    fn trace(&self, level: Level) {
        self.info().trace(level)
    }
}

impl Trace for crate::fairing::Info {
    fn trace(&self, level: Level) {
        event!(level, "fairing", name = self.name, kind = %self.kind)
    }
}

impl Trace for figment::error::Kind {
    fn trace(&self, _: Level) {
        use figment::error::{OneOf as V, Kind::*};

        match self {
            Message(message) => error!(message),
            InvalidType(actual, expected) => error!(%actual, expected, "invalid type"),
            InvalidValue(actual, expected) => error!(%actual, expected, "invalid value"),
            InvalidLength(actual, expected) => error!(%actual, expected, "invalid length"),
            UnknownVariant(actual, v) => error!(actual, expected = %V(v), "unknown variant"),
            UnknownField(actual, v) => error!(actual, expected = %V(v), "unknown field"),
            UnsupportedKey(actual, v) => error!(%actual, expected = &**v, "unsupported key"),
            MissingField(value) => error!(value = &**value, "missing field"),
            DuplicateField(value) => error!(value, "duplicate field"),
            ISizeOutOfRange(value) => error!(value, "out of range signed integer"),
            USizeOutOfRange(value) => error!(value, "out of range unsigned integer"),
            Unsupported(value) => error!(%value, "unsupported type"),
        }
    }
}

impl Trace for figment::Error {
    fn trace(&self, _: Level) {
        for e in self.clone() {
            span_error!("config",
                key = (!e.path.is_empty()).then_some(&e.path).and_then(|path| {
                    let (profile, metadata) = (e.profile.as_ref()?, e.metadata.as_ref()?);
                    Some(metadata.interpolate(profile, path))
                }),
                source.name = e.metadata.as_ref().map(|m| &*m.name),
                source.source = e.metadata.as_ref().and_then(|m| m.source.as_ref()).map(display)
                => e.kind.trace_error());
        }
    }
}

impl Trace for Header<'_> {
    fn trace(&self, level: Level) {
        event!(level, "header", name = self.name().as_str(), value = self.value());
    }
}

impl Trace for route::Outcome<'_> {
    fn trace(&self, level: Level) {
        event!(level, "outcome",
            outcome = match self {
                Self::Success(..) => "success",
                Self::Error(..) => "error",
                Self::Forward(..) => "forward",
            },
            status = match self {
                Self::Success(r) => r.status().code,
                Self::Error(s) => s.code,
                Self::Forward((_, s)) => s.code,
            },
        )
    }
}

impl Trace for Response<'_> {
    fn trace(&self, level: Level) {
        event!(level, "response", status = self.status().code);
    }
}

impl Trace for Error {
    fn trace(&self, level: Level) {
        self.kind.trace(level);
    }
}

impl Trace for Sentry {
    fn trace(&self, level: Level) {
        let (file, line, col) = self.location;
        event!(level, "sentry",
            type_name = self.type_name,
            location = %Formatter(|f| write!(f, "{file}:{line}:{col}"))
        );
    }
}

impl Trace for Request<'_> {
    fn trace(&self, level: Level) {
        event!(level, "request", method = %self.method(), uri = %self.uri())
    }
}

impl Trace for ConnectionMeta {
    fn trace(&self, level: Level) {
        event!(level, "connection",
            endpoint = self.peer_endpoint.as_ref().map(display),
            certs = self.peer_certs.is_some(),
        )
    }
}

impl Trace for ErrorKind {
    fn trace(&self, level: Level) {
        use ErrorKind::*;

        fn try_downcast<'a, T>(error: &'a (dyn StdError + 'static)) -> Option<&'a T>
            where T: StdError + 'static
        {
            error.downcast_ref().or_else(|| error.source()?.downcast_ref())
        }

        match self {
            Bind(endpoint, error) => {
                if let Some(e) = try_downcast::<crate::Error>(&**error) {
                    e.trace(level);
                } else if let Some(e) = try_downcast::<figment::Error>(&**error) {
                    e.trace(level);
                } else {
                    event!(level, "error::bind",
                        reason = %error,
                        endpoint = endpoint.as_ref().map(display),
                        "binding to network interface failed"
                    )
                }
            }
            Io(reason) => event!(level, "error::io", %reason, "i/o error"),
            Config(error) => error.trace(level),
            Collisions { routes, catchers }=> {
                span!(level, "collision",
                    route.pairs = routes.len(),
                    catcher.pairs = catchers.len(),
                    "colliding items detected"
                ).in_scope(|| {
                    for (a, b) in routes {
                        span!(level, "colliding route pair").in_scope(|| {
                            a.trace(level);
                            b.trace(level);
                        })
                    }

                    for (a, b) in catchers {
                        span!(level, "colliding catcher pair").in_scope(|| {
                            a.trace(level);
                            b.trace(level);
                        })
                    }

                    span!(Level::INFO, "collisions can usually be resolved by ranking items");
                });
            }
            FailedFairings(fairings) => {
                let span = span!(level, "failed ignite fairings", count = fairings.len());
                span.in_scope(|| fairings.iter().trace_all(level));
            },
            SentinelAborts(sentries) => {
                let span = span!(level, "sentries", "sentry launch abort");
                span.in_scope(|| sentries.iter().trace_all(level));
            }
            InsecureSecretKey(profile) => event!(level, "insecure_key", %profile,
                "secrets enabled in a non-debug profile without a stable `secret_key`\n\
                disable the `secrets` feature or configure a `secret_key`"
            ),
            Liftoff(_, reason) => event!(level, "panic", %reason, "liftoff fairing failed"),
            Shutdown(_) => event!(level, "shutdown", "shutdown failed"),
        }
    }
}
