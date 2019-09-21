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

// TODO Specify origins.
// TODO Documentation
// TODO Good default values for headers, etc.
// TODO Perhaps clone the settings or something.
// TODO Add a marker parameter "AllowOrigin" or "DenyOrigin"?  See what other cors libraries (node, sprint) have done.  What is the default?  What is the verb?

#[derive(Debug)]
pub struct CorsFairing {
    headers: Vec<String>,
    origin: Origin
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

        let mut uri_methods : HashMap<String, Vec<Method>> = HashMap::new();
        for route in rocket.routes()
        {
            let methods = uri_methods.entry(route.uri.path().to_string()).or_insert(Vec::new());
            methods.push(route.method);
        }


        let mut new_routes:Vec<Route> = Vec::new();
        for (uri, methods) in uri_methods.iter() {
            let options_handler = OptionsHandler::new(methods.clone(), self.headers.clone());

            let preflight = Route::new(Method::Options, uri, options_handler);
            new_routes.push(preflight);
        }
        
        rocket = rocket.mount("/", new_routes);
        Ok(rocket)
    }

    
    #[allow(unused_variables)]
    fn on_request(&self, request: &mut Request<'_>, data: &Data) {
        unimplemented!();
    }


    #[allow(unused_variables)]
    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        let origin_str = match self.origin {
            Origin::Any => "*".to_string(),
            Origin::Explicit(ref uri) => uri.to_string()
        };

        response.set_header(Header::new("Access-Control-Allow-Origin", origin_str));
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

pub struct CorsFairingBuilder {
    headers: Vec<String>,
    origin: Option<Origin>
}

#[derive(Debug, PartialEq)]
pub enum CorsFairingError {
    /// If the programmer has not chosen any origin then an error will be returned.
    NoOrigin
}

impl CorsFairingBuilder {
    pub fn new() -> CorsFairingBuilder {
        CorsFairingBuilder{
            headers: Vec::new(),
            origin: None
        }
    }

    pub fn build(self) -> Result<CorsFairing,CorsFairingError> {
        match self.origin {
            Some(origin) => {
                Ok(CorsFairing{
                    headers: self.headers,
                    origin: origin
                })
            },
            None => Result::Err(CorsFairingError::NoOrigin)
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
