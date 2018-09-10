use super::rocket;
use rocket::local::Client;
use rocket::http::tls::pemfile;

use std::fs::OpenOptions;
use std::io::BufReader;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn hello_world() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);

    // Open client's certificate
    let cert_file = OpenOptions::new()
        .read(true)
        .open("private/cert.pem")
        .expect("Openning file private/cert.pem failed.");

    // Generate pemfile from client's certificate
    let mut reader = BufReader::new(cert_file);
    let cert = pemfile::certs(&mut reader)
        .expect("Reading in pem file failed.");

    let client = Client::new(rocket)
        .expect("Generate a client for rocket failed.");

    // Create a SocketAddr to use as the client's address
    let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket =  SocketAddr::new(ip_addr, 8000);

    // Dispatch message with client's certificate and address
    let mut response = client
        .get("/")
        .certificate(cert[0].clone())
        .remote(socket)
        .dispatch();

    assert_eq!(response.body_string(), Some("Hello, MTLS world, localhost!".into()));
}
