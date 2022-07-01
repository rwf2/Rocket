use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::State;
use rocket::response::content::RawHtml;
use rocket::fairing::AdHoc;

struct HitCount(AtomicUsize);

#[get("/")]
fn index(hit_count: &State<HitCount>) -> RawHtml<String> {
    let count = hit_count.0.fetch_add(1, Ordering::Relaxed) + 1;
    RawHtml(format!("Your visit is recorded!<br /><br />Visits: {}", count))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managed Hit Count", |rocket| async {
        rocket.mount("/count", routes![index])
            .manage(HitCount(AtomicUsize::new(0)))
    })
}
