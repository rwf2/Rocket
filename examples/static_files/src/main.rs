extern crate rocket;
extern crate rocket_contrib;

#[cfg(test)] mod tests;

use rocket_contrib::serve::StaticFiles;

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", StaticFiles::from("static"))
}
