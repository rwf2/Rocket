#![cfg_attr(all(not(honggfuzz), not(afl)), no_main)]

use arbitrary::{Arbitrary, Unstructured, Result, Error};

use rocket::http::QMediaType;
use rocket::local::blocking::{LocalRequest, Client};
use rocket::http::{Method, Accept, ContentType, MediaType, uri::Origin};
use rocket::route::{Route, RouteUri, dummy_handler};

#[derive(Arbitrary)]
struct ArbitraryRequestData<'a> {
    method: ArbitraryMethod,
    origin: ArbitraryOrigin<'a>,
    format: Result<ArbitraryAccept, ArbitraryContentType>,
}

#[derive(Arbitrary)]
struct ArbitraryRouteData<'a> {
    method: ArbitraryMethod,
    uri: ArbitraryRouteUri<'a>,
    format: Option<ArbitraryMediaType>,
}

impl std::fmt::Debug for ArbitraryRouteData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArbitraryRouteData")
            .field("method", &self.method.0)
            .field("base", &self.uri.0.base())
            .field("unmounted", &self.uri.0.unmounted().to_string())
            .field("uri", &self.uri.0.to_string())
            .field("format", &self.format.as_ref().map(|v| v.0.to_string()))
            .finish()
    }
}

impl std::fmt::Debug for ArbitraryRequestData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArbitraryRequestData")
            .field("method", &self.method.0)
            .field("origin", &self.origin.0.to_string())
            .field("format", &self.format.as_ref()
                .map_err(|v| v.0.to_string())
                .map(|v| v.0.to_string()))
            .finish()
    }
}

impl<'c, 'a: 'c> ArbitraryRequestData<'a> {
    fn into_local_request(self, client: &'c Client) -> LocalRequest<'c> {
        let mut req = client.req(self.method.0, self.origin.0);
        match self.format {
            Ok(accept) => req.add_header(accept.0),
            Err(content_type) => req.add_header(content_type.0),
        }

        req
    }
}

impl<'a> ArbitraryRouteData<'a> {
    fn into_route(self) -> Route {
        let mut r = Route::ranked(0, self.method.0, &self.uri.0.to_string(), dummy_handler);
        r.format = self.format.map(|f| f.0);
        r
    }
}

struct ArbitraryMethod(Method);

struct ArbitraryOrigin<'a>(Origin<'a>);

struct ArbitraryAccept(Accept);

struct ArbitraryContentType(ContentType);

struct ArbitraryMediaType(MediaType);

struct ArbitraryRouteUri<'a>(RouteUri<'a>);

impl<'a> Arbitrary<'a> for ArbitraryMethod {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let all_methods = &[
            Method::Get, Method::Put, Method::Post, Method::Delete, Method::Options,
            Method::Head, Method::Trace, Method::Connect, Method::Patch
        ];

        Ok(ArbitraryMethod(*u.choose(all_methods)?))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, None)
    }
}

impl<'a> Arbitrary<'a> for ArbitraryOrigin<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let string = u.arbitrary::<&str>()?;
        if string.is_empty() {
            return Err(Error::NotEnoughData);
        }

        Origin::parse(string)
            .map(ArbitraryOrigin)
            .map_err(|_| Error::IncorrectFormat)
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, None)
    }
}

impl<'a> Arbitrary<'a> for ArbitraryAccept {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let media_type: ArbitraryMediaType = u.arbitrary()?;
        Ok(Self(Accept::new(QMediaType(media_type.0, None))))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        ArbitraryMediaType::size_hint(depth)
    }
}

impl<'a> Arbitrary<'a> for ArbitraryContentType {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let media_type: ArbitraryMediaType = u.arbitrary()?;
        Ok(ArbitraryContentType(ContentType(media_type.0)))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        ArbitraryMediaType::size_hint(depth)
    }
}

impl<'a> Arbitrary<'a> for ArbitraryMediaType {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let known = [
            "txt", "html", "htm", "xml", "opf", "xhtml", "csv", "js", "css", "json",
            "png", "gif", "bmp", "jpeg", "jpg", "webp", "avif", "svg", "ico", "flac", "wav",
            "webm", "weba", "ogg", "ogv", "pdf", "ttf", "otf", "woff", "woff2", "mp3", "mp4",
            "mpeg4", "wasm", "aac", "ics", "bin", "mpg", "mpeg", "tar", "gz", "tif", "tiff", "mov",
            "zip", "cbz", "cbr", "rar", "epub", "md", "markdown"
        ];

        let choice = u.choose(&known[..])?;
        let known = MediaType::from_extension(choice).unwrap();

        let top = u.ratio(1, 100)?.then_some("*".into()).unwrap_or(known.top().to_string());
        let sub = u.ratio(1, 100)?.then_some("*".into()).unwrap_or(known.sub().to_string());
        let params = u.ratio(1, 10)?
            .then_some(vec![])
            .unwrap_or(known.params().map(|(k, v)| (k.to_string(), v.to_owned())).collect());

        let media_type = MediaType::new(top, sub).with_params(params);
        Ok(ArbitraryMediaType(media_type))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (3, None)
    }
}

impl<'a> Arbitrary<'a> for ArbitraryRouteUri<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let (base, path) = (u.arbitrary::<&str>()?, u.arbitrary::<&str>()?);
        if base.is_empty() || path.is_empty() {
            return Err(Error::NotEnoughData);
        }

        RouteUri::try_new(base, path)
            .map(ArbitraryRouteUri)
            .map_err(|_| Error::IncorrectFormat)
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (2, None)
    }
}

type TestData<'a> = (
    ArbitraryRouteData<'a>,
    ArbitraryRouteData<'a>,
    ArbitraryRequestData<'a>
);

fn fuzz((route_a, route_b, req): TestData<'_>) {
    let rocket = rocket::custom(rocket::Config {
        workers: 2,
        log_level: rocket::log::LogLevel::Off,
        cli_colors: false,
        ..rocket::Config::debug_default()
    });

    let client = Client::untracked(rocket).expect("debug rocket is okay");
    let (route_a, route_b) = (route_a.into_route(), route_b.into_route());
    let local_request = req.into_local_request(&client);
    let request = local_request.inner();

    if route_a.matches(request) && route_b.matches(request) {
        assert!(route_a.collides_with(&route_b));
        assert!(route_b.collides_with(&route_a));
    }
}

#[cfg(all(not(honggfuzz), not(afl)))]
libfuzzer_sys::fuzz_target!(|data: TestData| fuzz(data));

#[cfg(honggbuzz)]
fn main() {
    loop {
        honggfuzz::fuzz!(|data: TestData| fuzz(data));
    }
}

#[cfg(afl)]
fn main() {
    afl::fuzz!(|data: TestData| fuzz(data));
}
