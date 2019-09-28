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
use rocket::response::Responder;
use rocket::Data;
use rocket::Request;
use rocket::Response;
use rocket::Rocket;
use rocket::Route;
use rocket::Handler;
use rocket::handler::Outcome;
use rocket::http::Header;
use rocket::http::Method;
use rocket::http::Status;

struct CorsContext {
    origin: String
}

#[derive(Debug)]
pub struct CorsFairing {
    config: Option<CorsFairingConfig>
}

impl CorsFairing {
    pub fn new() -> CorsFairing {
        CorsFairing {
            config: None
        }
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

        // TODO Factor this configuration out.  I should be able to reuse most of this configuration within
        // code between a test and the regular Rocket configuration.
        let (origin, headers) = match &self.config {
            Some(ref config) => {
                let origin = match &config.origin {
                    Some(ref origin) => match origin {
                        Origin::Any => String::from("*"),
                        Origin::Explicit(s) => s.clone()
                    },
                    None => {
                        warn_!("Bad headers.");

                        String::from("")
                    }
                };

                (origin, config.headers.clone())
            },
            None => {
                if let Ok(cors_table) = rocket.config().get_table("cors") {
                    let origin = match cors_table.get("allow_origin") {
                        Some(origin_value) => match origin_value {
                            Value::String(s) => s.clone(),
                            _ => {
                                warn_!("\"allow_origin\" configuration entry is missing.");

                                String::from("")
                            }
                        },
                        None => {
                            warn_!("Missing allow origin");

                            String::from("")
                        }
                    };

                    let headers = match cors_table.get("allow-headers") {
                        Some(allow_headers_value) => {
                            match allow_headers_value {
                                Value::Array(x) => {
                                    let result: Vec<String> = x.iter()
                                        .filter_map(|val| match val { 
                                            rocket::config::Value::String(s) => Some(s.clone()),
                                            _ => None
                                        })
                                        .collect();

                                    result
                                },
                                _ => {
                                warn_!("Bad allow origin");

                                    Vec::new()
                                }
                            }
                            
                        },
                        _ => Vec::new()
                    };
                    
                    (origin, headers)
                } else {
                    (String::from(""), Vec::new())
                }
            }
        };

        /*
        let origin = match &self.config {
            Some(ref config) => match config.origin {
                Some(ref origin) => string_from_origin(&origin),
                None => origin_from_config(&rocket)
            },
            None => {
                origin_from_config(&rocket)
            }
        };        

        let headers = match &self.config {
            Some(ref config) => config.headers.clone(),
            None => {
                if let Ok(headers) = rocket.config().get_slice("allowed_cross_origin_headers") {
                    let result: Vec<String> = headers.iter()
                        .filter_map(|val| match val { 
                            rocket::config::Value::String(s) => Some(s.clone()),
                            _ => None
                        })
                        .collect();

                    result
                } else {
                    warn_!("Bad headers.");

                    Vec::new()
                }
            }
        };
        */

        let ctx = CorsContext {
            origin: origin
        };
        
        let mut uri_methods : HashMap<String, Vec<Method>> = HashMap::new();
        for route in rocket.routes()
        {
            let methods = uri_methods.entry(route.uri.path().to_string()).or_insert(Vec::new());
            methods.push(route.method);
        }


        let mut new_routes:Vec<Route> = Vec::new();
        for (uri, methods) in uri_methods.iter() {
            let options_handler = OptionsHandler::new(methods.clone(), headers.clone());

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

struct OptionsResponder {
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<String>
}

fn comma_list(strings: &Vec<String>) -> String {
    let mut list = String::new();
    for x in 0..strings.len() {
        list.push_str(&strings[x]);
        if x < strings.len() - 1 {
            list.push_str(", ");
        }
    }

    list
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {
        assert_eq!(comma_list(&vec![String::from("Hello")]), "Hello");
        assert_eq!(comma_list(&vec![String::from("Hello"), String::from("World")]), "Hello, World");
    }
}


impl<'r> Responder<'r> for OptionsResponder {
    fn respond_to(self, _request: &Request<'_>) -> rocket::response::Result<'r> {
        let mut meths: Vec<String> = self.allowed_methods.iter().map(|x| format!("{}", x)).collect();
        meths.sort();
        let methods = comma_list(&meths);
        let headers = comma_list(&self.allowed_headers);

        let mut response = Response::build();
        
        response.raw_header("Access-Control-Allow-Methods", methods);
        response.raw_header("Access-Control-Allow-Headers", headers);
        response.status(Status::Ok)
        .ok()
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
        let responder = OptionsResponder{
            allowed_methods: self.allowed_methods.clone(),
            allowed_headers: self.allowed_headers.clone()
        };

        Outcome::from(req, responder)
    }
}

#[derive(Debug)]
enum Origin {
    Any,
    Explicit(String)
}

#[derive(Debug)]
pub struct CorsFairingConfig {
    headers: Vec<String>,
    origin: Option<Origin>
}

impl CorsFairingConfig {
    pub fn new() -> CorsFairingConfig {
        CorsFairingConfig{
            headers: Vec::new(),
            origin: None
        }
    }

    pub fn fairing(self) -> CorsFairing {
        CorsFairing {
            config: Some(self)
        }
    }

    pub fn add_header(mut self, header: &str) -> Self {
        self.headers.push(String::from(header));
        self
    }

    pub fn any_origin(mut self) -> Self {
        self.origin = Some(Origin::Any);

        self
    }

    pub fn explicit_origin(mut self, uri: &str) -> Self {
        self.origin = Some(Origin::Explicit(uri.to_string()));        

        self
    }
}
