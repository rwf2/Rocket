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
// TODO Only set Access-Control-Allow-Methods, Access-Control-Allow-Headers headers on OPTIONS call
// TODO Options should not return anything

#[derive(Debug)]
pub struct CorsFairing {
    origins: Vec<String>
}

impl CorsFairing {
    pub fn new() -> CorsFairing{
        CorsFairing {
            origins: Vec::new()
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
        let options_handler = OptionsHandler::new(vec![String::from("PATCH"), String::from("PUT"), String::from("POST")]);


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
        //response.set_header(Header::new("Access-Control-Allow-Methods", "POST, PATCH, DELETE"));
        //response.set_header(Header::new("Access-Control-Allow-Headers", "content-type"));
    }
}

struct OptionsResponder {
    allowed_methods: Vec<String>
}

impl<'r> Responder<'r> for OptionsResponder {
    fn respond_to(self, request: &Request) -> rocket::response::Result<'r> {
        let mut methods = String::new();
        for x in 0..self.allowed_methods.len() {
            methods.push_str(&self.allowed_methods[x]);

            if x < self.allowed_methods.len() - 1 {
                methods.push_str(", ");
            }
        }
        let mut response = Response::build();
        
        response.raw_header("Access-Control-Allow-Methods", methods);
        response.raw_header("Access-Control-Allow-Headers", "content-type");
        response.status(Status::Ok)
        .ok()
    }
}

#[derive(Clone)]
struct OptionsHandler {
    allowed_methods: Vec<String>
}

impl OptionsHandler {
    pub fn new(allowed_methods: Vec<String>) -> OptionsHandler {
        OptionsHandler { 
            allowed_methods: allowed_methods
        }
    }
}

impl Handler for OptionsHandler {
    fn handle<'r>(&self, req: &'r Request, data: Data) -> Outcome<'r> {
        let responder = OptionsResponder{
            allowed_methods: self.allowed_methods.clone()
        };

        Outcome::from(req, responder)
    }
}
