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

// TODO Determine methods actually used and then only return them.
// TODO Specify origins.
// TODO Documentation
// TODO Good default values for headers, etc.
// TODO Perhaps clone the settings or something.


#[derive(Debug)]
pub struct CorsFairing {
    headers: Vec<String>,
    origins: Vec<String>
}

impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Cors Fairing",
            kind: Kind::Attach | Kind::Response
        }
    }

    fn on_attach(&self, mut rocket:Rocket) -> Result<Rocket, Rocket> { 
        let mthds = vec![String::from("PATCH"), String::from("PUT"), String::from("POST")];

        let options_handler = OptionsHandler::new(mthds, self.headers.clone());


        let mut new_routes:Vec<Route> = Vec::new();


        for route in rocket.routes()
        {
            let uri_route = route.uri.path();
            let preflight = Route::new(Method::Options, uri_route, options_handler.clone());
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
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
    }
}

struct OptionsResponder {
    allowed_methods: Vec<String>,
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

        let methods = comma_list(&self.allowed_methods);
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
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>
}

impl OptionsHandler {
    pub fn new(allowed_methods: Vec<String>, allowed_headers: Vec<String>) -> OptionsHandler {
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


pub struct CorsFairingBuilder {
    headers: Vec<String>,
    origins: Vec<String>
}

impl CorsFairingBuilder {
    pub fn new() -> CorsFairingBuilder {
        CorsFairingBuilder{
            headers: Vec::new(),
            origins: Vec::new()
        }
    }

    pub fn build(self) -> CorsFairing {
        CorsFairing{
            headers: self.headers,
            origins: self.origins
        }
    }

    pub fn add_header(mut self, header: &str) -> Self {
        self.headers.push(String::from(header));
        self
    }
}
