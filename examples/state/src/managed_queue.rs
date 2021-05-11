use rocket::State;
use rocket::fairing::AdHoc;
use rocket::http::Status;

struct Tx(flume::Sender<String>);
struct Rx(flume::Receiver<String>);

#[put("/push?<event>")]
fn push(event: String, tx: &State<Tx>) -> Result<(), Status> {
    tx.0.try_send(event).map_err(|_| Status::ServiceUnavailable)
}

#[get("/pop")]
fn pop(rx: &State<Rx>) -> Option<String> {
    rx.0.try_recv().ok()
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managed Queue", |rocket| async {
        let (tx, rx) = flume::bounded(32);
        rocket.mount("/queue", routes![push, pop])
            .manage(Tx(tx))
            .manage(Rx(rx))
    })
}
