use std::fmt;
use std::convert::From;

use term_painter::ToStyle;
use term_painter::Color::*;

use codegen::StaticRouteInfo;
use handler::Handler;
use http::{Method, MediaType};
use http::uri::URI;

/// A route: a method, its handler, path, rank, and format/media type.
pub struct Route {
    /// The method this route matches against.
    pub method: Method,
    /// A function that should be called when the route matches.
    pub handler: Handler,
    /// The path (in Rocket format) that should be matched against.
    pub path: URI<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The media type this route matches against.
    pub format: Option<MediaType>,
}

#[inline(always)]
fn default_rank(uri: &URI) -> isize {
    // static path, query = -4; static path, no query = -3
    // dynamic path, query = -2; dynamic path, no query = -1
    match (!uri.path().contains('<'),  uri.query().is_some()) {
        (true, true) => -4,
        (true, false) => -3,
        (false, true) => -2,
        (false, false) => -1,
    }
}

impl Route {
    /// Creates a new route with the method, path, and handler.
    ///
    /// The rank of the route will be `0` if the path contains no dynamic
    /// segments, and `1` if it does.
    pub fn new<S>(m: Method, path: S, handler: Handler) -> Route
        where S: AsRef<str>
    {
        let uri = URI::from(path.as_ref().to_string());
        Route {
            method: m,
            handler: handler,
            rank: default_rank(&uri),
            path: uri,
            format: None,
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
            format: None,
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
    pub(crate) fn get_param_indexes(&self, uri: &URI) -> Vec<(usize, usize)> {
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

        if let Some(ref format) = self.format {
            write!(f, " {}", Yellow.paint(format))?;
        }

        Ok(())
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
        route.format = info.format.clone();
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}
