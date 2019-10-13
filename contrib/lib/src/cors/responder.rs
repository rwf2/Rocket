
use rocket::Request;

use rocket::Response;
use rocket::http::Method;
use rocket::http::Status;
use rocket::response::Responder;

pub struct PreflightCors {
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>
}

impl<'r> Responder<'r> for PreflightCors {
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
