#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::Data;

use std::io::Read;

const RESPONSE_BODY: &'static str = "Hello world";

#[post("/", data = "<data>")]
fn index(data: Data) -> String {
    let mut read = data.open();
    let mut buf = [0; 10];
    read.read(&mut buf).unwrap();

    RESPONSE_BODY.into()
}

mod http_keep_alive_tests {
    //! run tests with
    //! cargo test --test http_keep_alive -- --ignored
    use rocket;
    use rocket::config::{Config, Environment};
    use rocket::fairing::{Fairing, Info, Kind};
    use rocket::Rocket;

    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::str::from_utf8;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    const STARTING_PORT: u16 = 44406;
    static CURRENT_PORT: AtomicUsize = AtomicUsize::new(STARTING_PORT as usize);

    struct LaunchFairing(Arc<Barrier>);

    impl Fairing for LaunchFairing {
        fn info(&self) -> Info {
            Info {
                name: "Launch Fairing",
                kind: Kind::Launch,
            }
        }

        fn on_launch(&self, _: &Rocket) {
            let LaunchFairing(barrier) = self;
            barrier.wait();
        }
    }

    fn launch_rocket<'a>() -> u16 {
        let port = CURRENT_PORT.fetch_add(1, Ordering::SeqCst);

        let config = Config::build(Environment::Development)
            .port(port as u16)
            .workers(1)
            .unwrap();

        let barrier = Arc::new(Barrier::new(2));
        let launch_fairing = LaunchFairing(barrier.clone());

        thread::spawn(move || {
            rocket::custom(config)
                .attach(launch_fairing)
                .mount("/", routes![super::index])
                .launch();
        });
        barrier.wait();

        port as u16
    }

    fn connect_to_rocket(port: u16) -> TcpStream {
        let tcp = TcpStream::connect(("localhost", port)).expect(&format!(
            "could not connect to rocket server on port {}",
            port
        ));

        let timeout = Duration::new(1, 0);
        tcp.set_read_timeout(Some(timeout))
            .expect("failed to set read timeout on tcp stream");
        tcp.set_write_timeout(Some(timeout))
            .expect("failed to set write timeout on tcp stream");

        tcp
    }

    fn write_all<T: Write>(tcp: &mut T, data: &[u8]) -> ::std::io::Result<()> {
        let mut pos = 0;
        loop {
            let n = tcp.write(&data[pos..])?;
            pos += n;
            if pos >= data.len() {
                break;
            }
            if n == 0 {
                let error =
                    ::std::io::Error::new(::std::io::ErrorKind::Other, "tcp read 0 but isn't done");
                return Err(error);
            }
        }
        Ok(())
    }

    fn read_until<T: Read>(tcp: &mut T, byte: u8, buf: &mut Vec<u8>) -> ::std::io::Result<()> {
        let mut ibuf: [u8; 1] = [0; 1];
        let mut retries = 0;
        loop {
            let n = tcp.read(&mut ibuf)?;

            if n == 0 {
                if retries >= 10 {
                    let error = ::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "read_until: tcp read 0 for 200ms",
                    );
                    return Err(error);
                }
                ::std::thread::sleep(Duration::new(0, 20));
                retries += 1;
            } else {
                retries = 0;
                buf.extend(&ibuf);
                if ibuf[0] == byte {
                    break;
                }
            }
        }
        Ok(())
    }

    fn read_exact<T: Read>(tcp: &mut T, buf: &mut [u8]) -> ::std::io::Result<()> {
        let mut ibuf: [u8; 1] = [0; 1];
        let mut retries = 0;
        let mut i = 0;
        loop {
            let n = tcp.read(&mut ibuf)?;

            if n == 0 {
                if retries >= 10 {
                    let error = ::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "read_exact: tcp read 0 for 200ms",
                    );
                    return Err(error);
                }
                ::std::thread::sleep(Duration::new(0, 20));
                retries += 1;
            } else {
                retries = 0;
                buf[i] = ibuf[0];

                i += 1;
                if i >= buf.len() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn send_request<T: Write>(tcp: &mut T, body: &[u8]) -> ::std::io::Result<()> {
        write_all(tcp, b"POST / HTTP/1.1\n")?;
        write_all(tcp, b"Host: localhost:8000\n")?;
        write_all(tcp, b"Content-Type: text/plain\n")?;
        // content length is required for the connection to be kept alive
        // so we can test that the remaining body data isn't interpreted as another
        // request
        write_all(tcp, format!("Content-Length: {}\n", body.len()).as_bytes())?;
        write_all(tcp, b"\n")?;

        write_all(tcp, body)?;

        Ok(())
    }

    fn send_big_request<T: Write>(tcp: &mut T, body_size: usize) -> ::std::io::Result<()> {
        write_all(tcp, b"POST / HTTP/1.1\n")?;
        write_all(tcp, b"Host: localhost:8000\n")?;
        write_all(tcp, b"Content-Type: text/plain\n")?;
        // content length is required for the connection to be kept alive
        // so we can test that the remaining body data isn't interpreted as another
        // request
        write_all(tcp, format!("Content-Length: {}\n", body_size).as_bytes())?;
        write_all(tcp, b"\n")?;

        for _ in 0..body_size / 10 {
            write_all(tcp, b"RRRRRRRRRR")?;
        }
        for _ in 0..body_size % 10 {
            write_all(tcp, b"R")?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    struct ResponseInfo {
        raw: Vec<u8>,
        body: Vec<u8>,
    }

    fn read_response<T: Read>(tcp: &mut T) -> ::std::io::Result<ResponseInfo> {
        let mut raw = Vec::new();
        let mut line = Vec::new();

        let mut content_length: usize = 0;
        loop {
            line.clear();
            read_until(tcp, b'\n', &mut line)?;
            raw.extend_from_slice(&line);
            if line == b"\r\n" {
                // end of headers
                break;
            }
            let sep_pos = match line.iter().position(|c| *c == b':') {
                Some(i) => i,
                None => continue,
            };
            let (header, value) = line.split_at(sep_pos);
            let header = from_utf8(header).expect("header utf8 decode error").trim();
            // remove leading : and trim whitespace
            let value = from_utf8(value).expect("header utf8 decode error")[1..].trim();
            if header.to_lowercase() == "content-length" {
                content_length = value
                    .parse()
                    .expect("Could not parse content length header");
            }
        }

        let mut body = Vec::new();
        // body.extend(RESPONSE_BODY);
        body.resize(content_length, 0);
        read_exact(tcp, body.as_mut_slice())?;

        Ok(ResponseInfo {
            raw: raw,
            body: body,
        })
    }

    #[test]
    #[ignore]
    fn keep_alive_works() {
        let rocket_port = launch_rocket();

        let mut tcp = connect_to_rocket(rocket_port);

        // a number of bytes that will get read completely
        const SEND_BYTES: usize = 10;

        let body = [b'R'; SEND_BYTES];
        send_request(&mut tcp, &body).expect("send_request failed");

        let resp = read_response(&mut tcp).expect("failed to recieve response from server");

        assert_eq!(&super::RESPONSE_BODY.as_bytes(), &resp.body.as_slice());

        // send another request to confirm keep alive works
        send_request(&mut tcp, &body).expect("second send_request failed");

        let resp = read_response(&mut tcp).expect("failed to recieve second response from server");

        assert_eq!(&super::RESPONSE_BODY.as_bytes(), &resp.body.as_slice());
    }

    #[test]
    #[ignore]
    fn responds_and_closes_when_body_too_large() {
        let rocket_port = launch_rocket();

        let mut tcp = TcpStream::connect(("localhost", rocket_port)).expect(&format!(
            "could not connect to rocket server on port {}",
            rocket_port
        ));

        // a number of bytes that won't get read completely
        // connection should close during/after the first request
        const SEND_BYTES: usize = 15000;

        // this request may not complete depending on timing/buffer sizes
        send_big_request(&mut tcp, SEND_BYTES).ok();

        let resp = read_response(&mut tcp).expect("failed to recieve response from server");

        assert_eq!(&super::RESPONSE_BODY.as_bytes(), &resp.body.as_slice());

        // send another request and ensure there is no response to confirm the connection is closed
        // send_request may succeed depending on timing and available space in buffers but won't actually get processed by rocket
        // send request will fail more reliably with larger body sizes
        println!(
            "send request is ok {:?}",
            send_request(&mut tcp, b"8888").is_ok()
        );

        // ensure the server doesn't send any additional data it shouldn't
        let mut buf = [0; 10];
        match tcp.read(&mut buf) {
            Ok(0) => {
                println!("read returned 0");
            }
            Ok(n) => {
                panic!("was able to read {} bytes", n);
            }
            Err(_) => {}
        }
    }

    #[test]
    #[ignore]
    fn wont_read_excess_data() {
        let rocket_port = launch_rocket();

        let mut tcp = TcpStream::connect(("localhost", rocket_port)).expect(&format!(
            "could not connect to rocket server on port {}",
            rocket_port
        ));

        // a number of bytes that probably won't get read completely
        // this is dependent on timing and the size of buffers
        // connection should close after the first request
        const SEND_BYTES: usize = 150000;

        assert!(send_big_request(&mut tcp, SEND_BYTES).is_err());
    }
}
