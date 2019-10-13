//! A fairing to implement automatic [Cross-origin resource sharing](https://en.wikipedia.org/wiki/Cross-origin_resource_sharing).
//! 
//! This fairing will automatically add appropriate Access-Control-* headers 
//! to every route and generate preflight routes (OPTIONS) where required.
//!
//!
//! # Enabling
//!
//! This module is only available when the 'cors' feature is enabled.  Enable
//! cors in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! features = ["cors"]
//! ```
//!
//! Configure the allowed origin in your Rocket.toml file.
//! 
//! ```toml
//! [development.cors]
//! allow_origin = "https://example.com/"
//! allow_headers = ["X-Foo", "X-Bar"]
//! ```
//! 
//! Then add the fairing to your rocket before launching.
//! 
//! ```rust
//! #![feature(proc_macro_hygiene)]
//! 
//! #[macro_use] extern crate rocket;
//! 
//! use rocket_contrib::cors::CorsFairing;
//! 
//! #[get("/")]
//! fn index() -> &'static str {
//!     "Hello, world!"
//! }
//!
//! fn main() {
//!     let rocket = rocket::ignite()
//!         .mount("/", routes![index])
//!         // Add CorsFairing *after* your routes.
//!         .attach(CorsFairing::new());
//! # if false { // We don't actually want to launch the server in an example.
//!     rocket.launch();
//! # }
//! }
//! ```
//!
use rocket::config::Value;
use rocket::fairing::Fairing;
use rocket::fairing::Info;
use rocket::fairing::Kind;
use rocket::Data;
use rocket::Request;
use rocket::Response;
use rocket::Rocket;
use rocket::Route;
use rocket::Handler;
use rocket::handler::Outcome;
use rocket::http::Header;
use rocket::http::Method;

pub mod config;
use config::CorsFairingConfig;

mod responder;
use responder::PreflightCors;

struct CorsContext {
    origin: String
}

#[derive(Debug)]
pub struct CorsFairing {
    provided_config: Option<CorsFairingConfig>
}

impl CorsFairing {
    pub fn new() -> CorsFairing {
        CorsFairing {
            provided_config: None
        }
    }

    pub fn with_config(cors_fairing_config: CorsFairingConfig) -> CorsFairing {
        CorsFairing {
            provided_config: Some(cors_fairing_config)
        }
    }
}

fn make_from_rocket_config(config:&super::rocket::Config) -> Result<CorsFairingConfig, String> {
    match config.get_table("cors") {
        Ok(cors_table) => {

            let origin = match cors_table.get("allow_origin") {
                Some(origin_value) => match origin_value {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err("\"allow_origin\" configuration entry is missing.".to_string());
                    }
                },
                None => {
                    return Err("Missing allow origin".to_string());
                }
            };

            let mut headers = Vec::new();
            if let Some(allow_headers_value) = cors_table.get("allow-headers") {
                if let Value::Array(allow_headers_array) = allow_headers_value {
                    allow_headers_array
                        .iter()
                        .filter_map(|val| match val { 
                            Value::String(s) => Some(s.clone()),
                            _ => None
                        })
                        .for_each(|x| headers.push(x));

                }
            };
        
            Ok(CorsFairingConfig {
                origin: origin,
                headers: headers
            })
        },
        Err(_) => Err("Cors configuration missing.  Add a [development.cors] table (or equivalent configuration) to your Rocket.toml.".to_string())
    }
}


impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Cors Fairing",
            kind: Kind::Attach | Kind::Response
        }
    }

    
    fn on_attach(&self, mut rocket:Rocket) -> Result<Rocket, Rocket> { 
        use std::collections::HashMap;

        let config = match &self.provided_config {
            Some(config) => config.clone(),
            None => {
                match make_from_rocket_config(rocket.config()) {
                    Ok(config) => config,
                    Err(msg) => {
                        error!("{}", msg);
                        // Early return because we don't have a working Cors configuration.
                        return Err(rocket)
                    }
                }
            }
        };

        let ctx = CorsContext {
            origin: config.origin.clone()
        };
        
        let mut uri_methods : HashMap<String, Vec<Method>> = HashMap::new();
        for route in rocket.routes()
        {
            let methods = uri_methods.entry(route.uri.path().to_string()).or_insert(Vec::new());
            methods.push(route.method);
        }


        let mut new_routes:Vec<Route> = Vec::new();
        for (uri, methods) in uri_methods.iter() {
            let options_handler = OptionsHandler::new(methods.clone(), config.headers.clone());

            let preflight = Route::new(Method::Options, uri, options_handler);
            new_routes.push(preflight);
        }
        
        rocket = rocket.mount("/", new_routes);
        Ok(rocket.manage(ctx))
    }

    
    #[allow(unused_variables)]
    fn on_request(&self, _: &mut Request<'_>, _: &Data) {
        unimplemented!();
    }


    #[allow(unused_variables)]
    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        let context = request
            .guard::<rocket::State<'_, CorsContext>>()
            .expect("CorsContext registered in on_attach");

        response.set_header(Header::new("Access-Control-Allow-Origin", context.origin.clone()));
    }
}

#[derive(Clone)]
struct OptionsHandler {
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<String>
}

impl OptionsHandler {
    pub fn new(allowed_methods: Vec<Method>, allowed_headers: Vec<String>) -> OptionsHandler {
        OptionsHandler { 
            allowed_methods: allowed_methods,
            allowed_headers: allowed_headers
        }
    }
}

impl Handler for OptionsHandler {
    fn handle<'r>(&self, req: &'r Request<'_>, _data: Data) -> Outcome<'r> {
        let responder = PreflightCors {
            allowed_methods: self.allowed_methods.clone(),
            allowed_headers: self.allowed_headers.clone()
        };

        Outcome::from(req, responder)
    }
}