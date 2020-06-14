// This example's illustration is the Rocket.toml file.
#[rocket::main]
async fn main() {
    let _ = rocket::ignite().launch().await;
}
