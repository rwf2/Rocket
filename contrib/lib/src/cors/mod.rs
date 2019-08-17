use rocket::fairing::Fairing;
use rocket::fairing::Info;
use rocket::fairing::Kind;
use rocket::Data;
use rocket::Request;
use rocket::Response;
use rocket::Rocket;

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

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> { 
        unimplemented!();
    }

    
    #[allow(unused_variables)]
    fn on_request(&self, request: &mut Request<'_>, data: &Data) {
        unimplemented!();
    }


    #[allow(unused_variables)]
    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        unimplemented!();
    }
}
