use handler::{Handler, ErrorHandler};
use http::{Method, MediaType};

pub struct StaticRouteInfo {
    pub name: &'static str,
    pub method: Method,
    pub path: &'static str,
    pub format: Option<MediaType>,
    pub handler: Handler,
    pub rank: Option<isize>,
}

pub struct StaticCatchInfo {
    pub code: u16,
    pub handler: ErrorHandler,
}
