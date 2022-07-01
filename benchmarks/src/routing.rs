use std::collections::hash_set::HashSet;

use criterion::{criterion_group, Criterion};

use rocket::{route, config, Request, Data, Route, Config};
use rocket::http::{Method, RawStr, ContentType, Accept, Status};
use rocket::local::blocking::{Client, LocalRequest};

fn dummy_handler<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
    route::Outcome::from(req, ()).pin()
}

fn parse_routes_table(table: &str) -> Vec<Route> {
    let mut routes = vec![];
    for line in table.split("\n").filter(|s| !s.is_empty()) {
        let mut components = line.split(" ");
        let method: Method = components.next().expect("c").parse().expect("method");
        let uri: &str = components.next().unwrap();

        let (mut rank, mut name, mut format) = (None, None, None);
        for component in components {
            match component {
                c if c.starts_with('[') => rank = c.trim_matches(&['[', ']'][..]).parse().ok(),
                c if c.starts_with('(') => name = Some(c.trim_matches(&['(', ')'][..])),
                c => format = c.parse().ok(),
            }
        }

        let mut route = Route::new(method, uri, dummy_handler);
        if let Some(rank) = rank {
            route.rank = rank;
        }

        route.format = format;
        route.name = name.map(|s| s.to_string().into());
        routes.push(route);
    }

    routes
}

fn generate_matching_requests<'c>(client: &'c Client, routes: &[Route]) -> Vec<LocalRequest<'c>> {
    fn staticify_segment(segment: &RawStr) -> &str {
        segment.as_str().trim_matches(&['<', '>', '.', '_'][..])
    }

    fn request_for_route<'c>(client: &'c Client, route: &Route) -> LocalRequest<'c> {
        let path = route.uri.origin.path()
            .raw_segments()
            .map(staticify_segment)
            .collect::<Vec<_>>()
            .join("/");

        let query = route.uri.origin.query()
            .map(|q| q.raw_segments())
            .into_iter()
            .flatten()
            .map(staticify_segment)
            .collect::<Vec<_>>()
            .join("&");

        let uri = format!("/{}?{}", path, query);
        let mut req = client.req(route.method, uri);
        if let Some(ref format) = route.format {
            if route.method.supports_payload() {
                req.add_header(ContentType::from(format.clone()));
            } else {
                req.add_header(Accept::from(format.clone()));
            }
        }

        req
    }

    routes.iter()
        .map(|route| request_for_route(client, route))
        .collect()
}

fn client(routes: Vec<Route>) -> Client {
    let config = Config {
        profile: Config::RELEASE_PROFILE,
        log_level: rocket::config::LogLevel::Off,
        cli_colors: false,
        shutdown: config::Shutdown {
            ctrlc: false,
            #[cfg(unix)]
            signals: HashSet::new(),
            ..Default::default()
        },
        ..Default::default()
    };

    match Client::untracked(rocket::custom(config).mount("/", routes)) {
        Ok(client) => client,
        Err(e) => {
            drop(e);
            panic!("bad launch")
        }
    }
}

pub fn bench_rust_lang_routes(c: &mut Criterion) {
    let table = include_str!("../static/rust-lang.routes");
    let routes = parse_routes_table(table);
    let client = client(routes.clone());
    let requests = generate_matching_requests(&client, &routes);
    c.bench_function("rust-lang.routes", |b| b.iter(|| {
        for request in requests.clone() {
            let response = request.dispatch();
            assert_eq!(response.status(), Status::Ok);
        }
    }));
}

pub fn bench_bitwarden_routes(c: &mut Criterion) {
    let table = include_str!("../static/bitwarden_rs.routes");
    let routes = parse_routes_table(table);
    let client = client(routes.clone());
    let requests = generate_matching_requests(&client, &routes);
    c.bench_function("bitwarden_rs.routes", |b| b.iter(|| {
        for request in requests.clone() {
            let response = request.dispatch();
            assert_eq!(response.status(), Status::Ok);
        }
    }));
}

criterion_group!(routing, bench_rust_lang_routes, bench_bitwarden_routes);
