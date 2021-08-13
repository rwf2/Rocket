#[cfg(unix)]
#[macro_use] extern crate rocket;

#[cfg(unix)]
#[get("/")]
fn hi() -> &'static str {
    "Hello, Unix user!"
}

#[cfg(unix)]
#[tokio::main]
async fn main() {
    use hyperlocal::UnixServerExt;

    let server = rocket::build()
        .mount("/", routes![hi]);
    let service = server
        .ignite()
        .await
        .expect("invalid server configuration")
        .into_service();
    let make_service_fn = hyper::service::make_service_fn(move |_conn| {
        let service = service.clone();
        async {
            Ok::<_, std::convert::Infallible>(service)
        }
    });
    hyper::Server::bind_unix("/tmp/rocket.sock")
        .expect("failed to bind to Unix domain socket")
        .serve(make_service_fn)
        .await
        .expect("serve error");
    // connect with:
    // curl --unix-socket /tmp/rocket.sock http://any_name_here/
}


#[cfg(not(unix))]
fn main() {
    eprintln!("This example is only supported on Unix-like systems");
    std::process::exit(1);
}