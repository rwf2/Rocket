use {Rocket, Request, Response, Data};
use fairing::{Fairing, Kind};

#[derive(Default)]
pub struct Fairings {
    all_fairings: Vec<Box<Fairing>>,
    attach_failure: bool,
    launch: Vec<&'static Fairing>,
    request: Vec<&'static Fairing>,
    response: Vec<&'static Fairing>,
}

impl Fairings {
    #[inline]
    pub fn new() -> Fairings {
        Fairings::default()
    }

    pub fn attach(&mut self, fairing: Box<Fairing>, mut rocket: Rocket) -> Rocket {
        // Get the kind information.
        let kind = fairing.info().kind;

        // Run the `on_attach` callback if this is an 'attach' fairing.
        if kind.is(Kind::Attach) {
            rocket = fairing.on_attach(rocket)
                .unwrap_or_else(|r| { self.attach_failure = true; r })
        }

        // The `Fairings` structure separates `all_fairings` into kind groups so
        // we don't have to search through all fairings and do a comparison at
        // runtime. We need references since a single structure can be multiple
        // kinds. The lifetime of that reference is really the lifetime of the
        // `Box` for referred fairing, but that lifetime is dynamic; there's no
        // way to express it. So we cheat and say that the lifetime is
        // `'static` and cast it here. For this to be safe, the following must
        // be preserved:
        //
        //  1) The references can never be exposed with a `'static` lifetime.
        //  2) The `Box<Fairing>` must live for the lifetime of the reference.
        //
        // We maintain these invariants by not exposing the references and never
        // deallocating `Box<Fairing>` structures. As such, the references will
        // always be valid. Note: `ptr` doesn't point into the `Vec`, so
        // reallocations there are irrelvant. Instead, it points into the heap.
        //
        // Also, we don't save attach fairings since we don't need them anymore.
        if !kind.is_exactly(Kind::Attach) {
            let ptr: &'static Fairing = unsafe { ::std::mem::transmute(&*fairing) };

            self.all_fairings.push(fairing);
            if kind.is(Kind::Launch) { self.launch.push(ptr); }
            if kind.is(Kind::Request) { self.request.push(ptr); }
            if kind.is(Kind::Response) { self.response.push(ptr); }
        }

        rocket
    }

    #[inline(always)]
    pub fn handle_launch(&self, rocket: &Rocket) {
        for fairing in &self.launch {
            fairing.on_launch(rocket);
        }
    }

    #[inline(always)]
    pub fn handle_request(&self, req: &mut Request, data: &Data) {
        for fairing in &self.request {
            fairing.on_request(req, data);
        }
    }

    #[inline(always)]
    pub fn handle_response(&self, request: &Request, response: &mut Response) {
        for fairing in &self.response {
            fairing.on_response(request, response);
        }
    }

    pub fn had_failure(&self) -> bool {
        self.attach_failure
    }

    pub fn pretty_print_counts(&self) {
        use yansi::Paint;

        if self.all_fairings.is_empty() {
            return
        }

        fn info_if_nonempty(kind: &str, fairings: &[&Fairing]) {
            let names: Vec<&str> = fairings.iter().map(|f| f.info().name).collect();
            info_!("{} {}: {}",
                   Paint::white(fairings.len()),
                   kind,
                   Paint::white(names.join(", ")));
        }

        info!("📡  {}:", Paint::purple("Fairings"));
        info_if_nonempty("launch", &self.launch);
        info_if_nonempty("request", &self.request);
        info_if_nonempty("response", &self.response);
    }
}
