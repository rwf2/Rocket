#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::net::SocketAddr;

#[get("/")]
fn get_ip(remote: SocketAddr) -> String {
    remote.to_string()
}

mod remote_rewrite_tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::{Header, Status};

    use std::net::SocketAddr;

    const KNOWN_IP: &'static str = "127.0.0.1:8000";

    fn check_ip(header: Option<Header<'static>>, ip: Option<String>) {
        let addr: SocketAddr = KNOWN_IP.parse().unwrap();

        let c = Client::new(rocket::ignite().mount("/", routes![get_ip])).unwrap();
        let mut response = match header {
            Some(header) => c.get("/").header(header).remote(addr).dispatch(),
            None => c.get("/").remote(addr).dispatch()
        };

        assert_eq!(response.status(), Status::Ok);
        let body = response.body_string();
        match ip {
            Some(ip) => assert_eq!(body, Some(format!("{}:{}", ip, addr.port()))),
            None => assert_eq!(body, Some(KNOWN_IP.into()))
        }
    }

    #[test]
    fn x_real_ip_rewrites() {
        let ip = "8.8.8.8";
        check_ip(Some(Header::new("X-Real-IP", ip)), Some(ip.to_string()));

        let ip = "129.120.111.200";
        check_ip(Some(Header::new("X-Real-IP", ip)), Some(ip.to_string()));
    }

    #[test]
    fn x_real_ip_rewrites_ipv6() {
        let ip = "2001:db8:0:1:1:1:1:1";
        check_ip(Some(Header::new("X-Real-IP", ip)), Some(format!("[{}]", ip)));

        let ip = "2001:db8::2:1";
        check_ip(Some(Header::new("X-Real-IP", ip)), Some(format!("[{}]", ip)));
    }

    #[test]
    fn uncased_header_rewrites() {
        let ip = "8.8.8.8";
        check_ip(Some(Header::new("x-REAL-ip", ip)), Some(ip.to_string()));

        let ip = "1.2.3.4";
        check_ip(Some(Header::new("x-real-ip", ip)), Some(ip.to_string()));
    }

    #[test]
    fn no_header_no_rewrite() {
        check_ip(Some(Header::new("real-ip", "?")), None);
        check_ip(None, None);
    }

    #[test]
    fn bad_header_doesnt_rewrite() {
        let ip = "092348092348";
        check_ip(Some(Header::new("X-Real-IP", ip)), None);

        let ip = "1200:100000:0120129";
        check_ip(Some(Header::new("X-Real-IP", ip)), None);

        let ip = "192.168.1.900";
        check_ip(Some(Header::new("X-Real-IP", ip)), None);
    }
}
