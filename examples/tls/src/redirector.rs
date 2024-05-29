//! Redirect all HTTP requests to HTTPs.

use rocket::http::Status;
use rocket::log::LogLevel;
use rocket::{route, Error, Request, Data, Route, Orbit, Rocket, Ignite, Config};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::response::Redirect;

#[derive(Debug, Copy, Clone)]
pub struct Redirector {
    pub port: u16
}

/// Managed state for the TLS server's configuration.
struct TlsConfig(Config);

impl Redirector {
    // Route function that gets call on every single request.
    fn redirect<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
        // TODO: Check the host against a whitelist!
        let config = req.rocket().state::<TlsConfig>().expect("managed config");
        if let Some(host) = req.host() {
            let domain = host.domain();
            let https_uri = match dbg!(config.0.port) {
                443 => format!("https://{domain}{}", req.uri()),
                port => format!("https://{domain}:{port}{}", req.uri()),
            };

            route::Outcome::from(req, Redirect::temporary(https_uri)).pin()
        } else {
            route::Outcome::from(req, Status::BadRequest).pin()
        }
    }

    // Launch an instance of Rocket than handles redirection on `self.port`.
    pub async fn try_launch(self, config: Config) -> Result<Rocket<Ignite>, Error> {
        use rocket::http::Method::*;

        // Adjust config for redirector: disable TLS, set port, disable logging.
        let redirector_config = Config {
            tls: None,
            port: self.port,
            log_level: LogLevel::Critical,
            ..config.clone()
        };

        // Build a vector of routes to `redirect` on `<path..>` for each method.
        let redirects = [Get, Put, Post, Delete, Options, Head, Trace, Connect, Patch]
            .into_iter()
            .map(|m| Route::new(m, "/<path..>", Self::redirect))
            .collect::<Vec<_>>();

        rocket::custom(redirector_config)
            .manage(TlsConfig(config))
            .mount("/", redirects)
            .launch()
            .await
    }
}

#[rocket::async_trait]
impl Fairing for Redirector {
    fn info(&self) -> Info {
        Info { name: "HTTP -> HTTPS Redirector", kind: Kind::Liftoff }
    }

    async fn on_liftoff(&self, rkt: &Rocket<Orbit>) {
        let (this, shutdown, config) = (*self, rkt.shutdown(), rkt.config().clone());
        let _ = rocket::tokio::spawn(async move {
            if let Err(e) = this.try_launch(config).await {
                error!("Failed to start HTTP -> HTTPS redirector.");
                info_!("Error: {}", e);
                error_!("Shutting down main instance.");
                shutdown.notify();
            }
        });
    }
}
