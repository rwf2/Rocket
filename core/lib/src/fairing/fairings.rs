use crate::{Rocket, Request, Response, Data};
use crate::fairing::{Fairing, Kind};

use yansi::Paint;

#[derive(Default)]
pub struct Fairings {
    all_fairings: Vec<Box<dyn Fairing>>,
    attach_failures: Vec<&'static str>,
    // The vectors below hold indices into `all_fairings`.
    launch: Vec<usize>,
    request: Vec<usize>,
    response: Vec<usize>,
}

impl Fairings {
    #[inline]
    pub fn new() -> Fairings {
        Fairings::default()
    }

    pub fn attach(&mut self, fairing: Box<dyn Fairing>, mut rocket: Rocket) -> Rocket {
        // Run the `on_attach` callback if this is an 'attach' fairing.
        let kind = fairing.info().kind;
        let name = fairing.info().name;
        if kind.is(Kind::Attach) {
            rocket = fairing.on_attach(rocket)
                .unwrap_or_else(|r| { self.attach_failures.push(name); r })
        }

        self.add(fairing);
        rocket
    }

    fn add(&mut self, fairing: Box<dyn Fairing>) {
        let kind = fairing.info().kind;
        if !kind.is_exactly(Kind::Attach) {
            let index = self.all_fairings.len();
            self.all_fairings.push(fairing);

            if kind.is(Kind::Launch) { self.launch.push(index); }
            if kind.is(Kind::Request) { self.request.push(index); }
            if kind.is(Kind::Response) { self.response.push(index); }
        }
    }

    pub fn append(&mut self, others: Fairings) {
        for fairing in others.all_fairings {
            self.add(fairing);
        }
    }

    #[inline(always)]
    pub fn handle_launch(&self, rocket: &Rocket) {
        for &i in &self.launch {
            self.all_fairings[i].on_launch(rocket);
        }
    }

    #[inline(always)]
    pub fn handle_request(&self, req: &mut Request<'_>, data: &Data) {
        for &i in &self.request {
            self.all_fairings[i].on_request(req, data);
        }
    }

    #[inline(always)]
    pub fn handle_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        for &i in &self.response {
            self.all_fairings[i].on_response(request, response);
        }
    }

    pub fn failures(&self) -> Option<&[&'static str]> {
        if self.attach_failures.is_empty() {
            None
        } else {
            Some(&self.attach_failures)
        }
    }

    fn info_for(&self, kind: &str, fairings: &[usize]) {
        if !fairings.is_empty() {
            let num = fairings.len();
            let names = fairings.iter().cloned()
                .map(|i| self.all_fairings[i].info().name)
                .collect::<Vec<_>>()
                .join(", ");

            info_!("{} {}: {}", Paint::default(num).bold(), kind, Paint::default(names).bold());
        }
    }

    pub fn pretty_print_counts(&self) {
        if !self.all_fairings.is_empty() {
            info!("{}{}:", Paint::masked("📡 "), Paint::magenta("Fairings"));
            self.info_for("launch", &self.launch);
            self.info_for("request", &self.request);
            self.info_for("response", &self.response);
        }
    }
}
