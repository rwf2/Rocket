use handler::{Handler, ErrorHandler};
use http::{MediaType};

pub struct StaticRouteInfo {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub format: Option<MediaType>,
    pub handler: Handler,
    pub rank: Option<isize>,
}

pub struct StaticCatchInfo {
    pub code: u16,
    pub handler: ErrorHandler,
}
