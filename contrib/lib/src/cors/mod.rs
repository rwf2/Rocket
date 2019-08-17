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
        let mut new_routes:Vec<Route> = Vec::new();

        for route in rocket.routes()
        {
            println!("{:?}", route.uri.path());
        }


        let r = Route::new(Method::Options, "/foo", OptionsHandler::new());
        new_routes.push(r);
        //Route::new(..)
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
        //unimplemented!();
    }
}

#[derive(Clone)]
struct OptionsHandler {
    
}

impl OptionsHandler {
    pub fn new() -> OptionsHandler {
        OptionsHandler { }
    }
}

impl Handler for OptionsHandler {
    fn handle<'r>(&self, req: &'r Request, data: Data) -> Outcome<'r> {
        Outcome::from(req, "hello world")
    }
}