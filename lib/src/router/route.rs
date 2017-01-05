use std::fmt;
use std::convert::From;

use super::Collider; // :D

use term_painter::ToStyle;
use term_painter::Color::*;

use codegen::StaticRouteInfo;
use handler::Handler;
use request::Request;
use http::{Method, ContentType};
use http::uri::URI;

/// A route: a method, its handler, path, rank, and format/content type.
pub struct Route {
    /// The method this route matches against.
    pub method: Method,
    /// A function that should be called when the route matches.
    pub handler: Handler,
    /// The path (in Rocket format) that should be matched against.
    pub path: URI<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The Content-Type this route matches against.
    pub format: ContentType,
}

fn default_rank(path: &str) -> isize {
    // The rank for a given path is 0 if it is a static route (it doesn't
    // contain any dynamic <segmants>) or 1 if it is dynamic.
    path.contains('<') as isize
}

impl Route {
    /// Creates a new route with the method, path, and handler.
    ///
    /// The rank of the route will be `0` if the path contains no dynamic
    /// segments, and `1` if it does.
    pub fn new<S>(m: Method, path: S, handler: Handler) -> Route
        where S: AsRef<str>
    {
        Route {
            method: m,
            handler: handler,
            rank: default_rank(path.as_ref()),
            path: URI::from(path.as_ref().to_string()),
            format: ContentType::Any,
        }
    }

    /// Creates a new route with the given rank, method, path, and handler.
    pub fn ranked<S>(rank: isize, m: Method, path: S, handler: Handler) -> Route
        where S: AsRef<str>
    {
        Route {
            method: m,
            path: URI::from(path.as_ref().to_string()),
            handler: handler,
            rank: rank,
            format: ContentType::Any,
        }
    }

    /// Sets the path of the route. Does not update the rank or any other
    /// parameters.
    pub fn set_path<S>(&mut self, path: S) where S: AsRef<str> {
        self.path = URI::from(path.as_ref().to_string());
    }

    // FIXME: Decide whether a component has to be fully variable or not. That
    // is, whether you can have: /a<a>b/ or even /<a>:<b>/
    // TODO: Don't return a Vec...take in an &mut [&'a str] (no alloc!)
    /// Given a URI, returns a vector of slices of that URI corresponding to the
    /// dynamic segments in this route.
    #[doc(hidden)]
    pub fn get_param_indexes(&self, uri: &URI) -> Vec<(usize, usize)> {
        let route_segs = self.path.segments();
        let uri_segs = uri.segments();
        let start_addr = uri.path().as_ptr() as usize;

        let mut result = Vec::with_capacity(self.path.segment_count());
        for (route_seg, uri_seg) in route_segs.zip(uri_segs) {
            let i = (uri_seg.as_ptr() as usize) - start_addr;
            if route_seg.ends_with("..>") {
                result.push((i, uri.path().len()));
                break;
            } else if route_seg.ends_with('>') {
                let j = i + uri_seg.len();
                result.push((i, j));
            }
        }

        result
    }
}

impl Clone for Route {
    fn clone(&self) -> Route {
        Route {
            method: self.method,
            handler: self.handler,
            rank: self.rank,
            path: self.path.clone(),
            format: self.format.clone(),
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Green.paint(&self.method), Blue.paint(&self.path))?;

        if self.rank > 1 {
            write!(f, " [{}]", White.paint(&self.rank))?;
        }

        if !self.format.is_any() {
            write!(f, " {}", Yellow.paint(&self.format))
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Route as fmt::Display>::fmt(self, f)
    }
}

#[doc(hidden)]
impl<'a> From<&'a StaticRouteInfo> for Route {
    fn from(info: &'a StaticRouteInfo) -> Route {
        let mut route = Route::new(info.method, info.path, info.handler);
        route.format = info.format.clone().unwrap_or(ContentType::Any);
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}

impl Collider for Route {
    fn collides_with(&self, b: &Route) -> bool {
        self.method == b.method
            && self.rank == b.rank
            && self.format.collides_with(&b.format)
            && self.path.collides_with(&b.path)
    }
}

impl<'r> Collider<Request<'r>> for Route {
    fn collides_with(&self, req: &Request<'r>) -> bool {
        self.method == req.method()
            && req.uri().collides_with(&self.path)
            // FIXME: On payload requests, check Content-Type. On non-payload
            // requests, check Accept.
            && req.content_type().collides_with(&self.format)
    }
}
