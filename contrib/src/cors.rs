use std::collections::HashSet;
use rocket::response::{self, Response, Responder};
use rocket::http::Method;

/// The CORS type, which implements `Responder`. This type allows
/// you to request resources from another domain.
///
/// # Usage
///
/// You can use the `CORS` type for you routes as a preflight like that:
///
/// ```rust,ignore
/// #[route(OPTIONS, "/user")]
/// fn cors_preflight() -> PreflightCORS {
///     CORS::preflight("http://somehost.com")
///         .methods(vec![Method::Options, Methods::Get])
///         .headers(vec!["Content-Type"])
/// }
/// ```
///
/// And then you can just simply do something like this:
/// 
/// ```rust,ignore
/// #[get("/user")]
/// fn user() -> CORS<String> {
///     CORS::any("Hello I'm User!".to_string())
/// }
/// ```
pub struct CORS<R> {
    responder: R,
    allow_origin: &'static str,
    allow_credentials: bool,
    expose_headers: HashSet<&'static str>,
    max_age: Option<usize>,
    allow_methods: HashSet<Method>,
    allow_headers: HashSet<&'static str>
}

pub type PreflightCORS = CORS<()>;

impl PreflightCORS {
    /// Consumes origin for which it will allow to use `CORS`
    /// and return a basic origin `CORS`
    pub fn preflight(origin: &'static str) -> PreflightCORS {
        CORS::origin((), origin)
    }
}

impl<'r, R: Responder<'r>> CORS<R> {
    /// Consumes responder and returns CORS with any origin
    pub fn any(responder: R) -> CORS<R> {
        CORS::origin(responder, "*")
    }

    /// Consumes the responder and origin and returns basic CORS
    pub fn origin(responder: R, origin: &'static str) -> CORS<R> {
        CORS {
            responder: responder,
            allow_origin: origin,
            allow_credentials: false,
            expose_headers: HashSet::new(),
            max_age: None,
            allow_methods: HashSet::new(),
            allow_headers: HashSet::new()
        }
    }

    /// Consumes the CORS, set allow_credentials to
    /// new value and returns changed CORS
    pub fn credentials(mut self, value: bool) -> CORS<R> {
        self.allow_credentials = value;
        self
    }

    /// Consumes the CORS, set expose_headers to
    /// passed headers and returns changed CORS
    pub fn exposed_headers(mut self, headers: &[&'static str]) -> CORS<R> {
        self.expose_headers = headers.into_iter().cloned().collect();
        self
    }

    /// Consumes the CORS, set max_age to
    /// passed value and returns changed CORS
    pub fn max_age(mut self, value: Option<usize>) -> CORS<R> {
        self.max_age = value;
        self
    }

    /// Consumes the CORS, set allow_methods to
    /// passed methods and returns changed CORS
    pub fn methods(mut self, methods: &[Method]) -> CORS<R> {
        self.allow_methods = methods.into_iter().cloned().collect();
        self
    }

    /// Consumes the CORS, set allow_headers to
    /// passed headers and returns changed CORS
    pub fn headers(mut self, headers: &[&'static str]) -> CORS<R> {
        self.allow_headers = headers.into_iter().cloned().collect();
        self
    }
}

impl <'r, R: Responder<'r>> Responder<'r> for CORS<R> {
    fn respond(self) -> response::Result<'r> {
        let mut response = Response::build_from(self.responder.respond()?)
            .raw_header("Access-Control-Allow-Origin", self.allow_origin)
            .finalize();

        if self.allow_credentials {
            response.set_raw_header("Access-Control-Allow-Credentials", "true");
        } else {
            response.set_raw_header("Access-Control-Allow-Credentials", "false");
        }

        if !self.expose_headers.is_empty() {
            let headers: Vec<_> = self.expose_headers.into_iter().collect();
            let headers = headers.join(", ");

            response.set_raw_header("Access-Control-Expose-Headers", headers);
        }

        if !self.allow_methods.is_empty() {
            let methods: Vec<_> = self.allow_methods
                .into_iter()
                .map(|m| m.as_str())
                .collect();
            let methods = methods.join(", ");

            response.set_raw_header("Access-Control-Allow-Methods", methods);
        }

        if self.max_age.is_some() {
            let max_age = self.max_age.unwrap();
            response.set_raw_header("Access-Control-Max-Age", max_age.to_string());
        }

        Ok(response)
    }
}
