use std::collections::HashSet;

use crate::{Rocket, Request, Response, Data, Build, Orbit};
use crate::fairing::{Fairing, Info, Kind};
use crate::log::PaintExt;

use yansi::Paint;

#[derive(Default)]
pub struct Fairings {
    // NOTE: This is a push-only vector due to the index-vectors below!
    all_fairings: Vec<Box<dyn Fairing>>,
    // Ignite fairings that have failed.
    failures: Vec<Info>,
    // The number of ignite fairings from `self.ignite` we've run.
    num_ignited: usize,
    // The vectors below hold indices into `all_fairings`.
    ignite: Vec<usize>,
    liftoff: Vec<usize>,
    request: Vec<usize>,
    response: Vec<usize>,
    shutdown: Vec<usize>,
}

macro_rules! iter {
    ($_self:ident . $kind:ident) => ({
        iter!($_self, $_self.$kind.iter()).map(|v| v.1)
    });
    ($_self:ident, $indices:expr) => ({
        let all_fairings = &$_self.all_fairings;
        $indices.filter_map(move |i| {
            debug_assert!(all_fairings.get(*i).is_some());
            let f = all_fairings.get(*i).map(|f| &**f)?;
            Some((*i, f))
        })
    })
}

impl Fairings {
    #[inline]
    pub fn new() -> Fairings {
        Fairings::default()
    }

    pub fn active(&self) -> impl Iterator<Item = &usize> {
        self.ignite.iter()
            .chain(self.liftoff.iter())
            .chain(self.request.iter())
            .chain(self.response.iter())
            .chain(self.shutdown.iter())
    }

    pub fn add(&mut self, fairing: Box<dyn Fairing>) {
        let this = &fairing;
        let this_info = this.info();
        if this_info.kind.is(Kind::Singleton) {
            // If we already ran a duplicate on ignite, then fail immediately.
            // There is no way to uphold the "only run last singleton" promise.
            //
            // How can this happen? Like this:
            //   1. Attach A (singleton).
            //   2. Attach B (any fairing).
            //   3. Ignite.
            //   4. A executes on_ignite.
            //   5. B executes on_ignite, attaches another A.
            //   6. --- (A would run if not for this code)
            let ignite_dup = iter!(self.ignite).position(|f| f.type_id() == this.type_id());
            if let Some(dup_ignite_index) = ignite_dup {
                if dup_ignite_index < self.num_ignited {
                    self.failures.push(this_info);
                    return;
                }
            }

            // Finds `k` in `from` and removes it if it's there.
            let remove = |k: usize, from: &mut Vec<usize>| {
                if let Ok(j) = from.binary_search(&k) {
                    from.remove(j);
                }
            };

            // Collect all of the active duplicates.
            let mut dups: Vec<usize> = iter!(self, self.active())
                .filter(|(_, f)| f.type_id() == this.type_id())
                .map(|(i, _)| i)
                .collect();

            // Reverse the dup indices so `remove` is stable given shifts.
            dups.sort(); dups.dedup(); dups.reverse();
            for i in dups {
                remove(i, &mut self.ignite);
                remove(i, &mut self.liftoff);
                remove(i, &mut self.request);
                remove(i, &mut self.response);
                remove(i, &mut self.shutdown);
            }
        }

        let index = self.all_fairings.len();
        self.all_fairings.push(fairing);
        if this_info.kind.is(Kind::Ignite) { self.ignite.push(index); }
        if this_info.kind.is(Kind::Liftoff) { self.liftoff.push(index); }
        if this_info.kind.is(Kind::Request) { self.request.push(index); }
        if this_info.kind.is(Kind::Response) { self.response.push(index); }
        if this_info.kind.is(Kind::Shutdown) { self.shutdown.push(index); }
    }

    pub fn append(&mut self, others: &mut Fairings) {
        for fairing in others.all_fairings.drain(..) {
            self.add(fairing);
        }
    }

    pub async fn handle_ignite(mut rocket: Rocket<Build>) -> Rocket<Build> {
        while rocket.fairings.num_ignited < rocket.fairings.ignite.len() {
            // We're going to move `rocket` while borrowing `fairings`...
            let mut fairings = std::mem::replace(&mut rocket.fairings, Fairings::new());
            for fairing in iter!(fairings.ignite).skip(fairings.num_ignited) {
                let info = fairing.info();
                rocket = match fairing.on_ignite(rocket).await {
                    Ok(rocket) => rocket,
                    Err(rocket) => {
                        fairings.failures.push(info);
                        rocket
                    }
                };

                fairings.num_ignited += 1;
            }

            // Note that `rocket.fairings` may now be non-empty since ignite
            // fairings could have added more fairings! Move them to the end.
            fairings.append(&mut rocket.fairings);
            rocket.fairings = fairings;
        }

        rocket
    }

    #[inline(always)]
    pub async fn handle_liftoff(&self, rocket: &Rocket<Orbit>) {
        let liftoff_futures = iter!(self.liftoff).map(|f| f.on_liftoff(rocket));
        futures::future::join_all(liftoff_futures).await;
    }

    #[inline(always)]
    pub async fn handle_request(&self, req: &mut Request<'_>, data: &mut Data<'_>) {
        for fairing in iter!(self.request) {
            fairing.on_request(req, data).await
        }
    }

    #[inline(always)]
    pub async fn handle_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        for fairing in iter!(self.response) {
            fairing.on_response(req, res).await;
        }
    }

    #[inline(always)]
    pub async fn handle_shutdown(&self, rocket: &Rocket<Orbit>) {
        let shutdown_futures = iter!(self.shutdown).map(|f| f.on_shutdown(rocket));
        futures::future::join_all(shutdown_futures).await;
    }

    pub fn audit(&self) -> Result<(), &[Info]> {
        match self.failures.is_empty() {
            true => Ok(()),
            false => Err(&self.failures)
        }
    }

    pub fn pretty_print(&self) {
        let active_fairings = self.active().collect::<HashSet<_>>();
        if !active_fairings.is_empty() {
            launch_meta!("{}{}:", Paint::emoji("📡 "), Paint::magenta("Fairings"));

            for (_, fairing) in iter!(self, active_fairings.into_iter()) {
                launch_meta_!("{} ({})", Paint::default(fairing.info().name).bold(),
                Paint::blue(fairing.info().kind).bold());
            }
        }
    }
}

impl std::fmt::Debug for Fairings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn debug_info<'a>(iter: impl Iterator<Item = &'a dyn Fairing>) -> Vec<Info> {
            iter.map(|f| f.info()).collect()
        }

        f.debug_struct("Fairings")
            .field("launch", &debug_info(iter!(self.ignite)))
            .field("liftoff", &debug_info(iter!(self.liftoff)))
            .field("request", &debug_info(iter!(self.request)))
            .field("response", &debug_info(iter!(self.response)))
            .field("shutdown", &debug_info(iter!(self.shutdown)))
            .finish()
    }
}
