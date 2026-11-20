#![cfg(feature = "memcache_pool")]

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rocket::figment::Figment;
use rocket_sync_db_pools::{database, memcache, r2d2::ManageConnection, Config, Error, Poolable};
use rocket_sync_db_pools::MemcacheConnectionManager as Manager;
use memcache::MemcacheError;

#[database("memcache")]
struct MemcacheDb(memcache::Client);

#[test]
fn invalid_url_durations() {
    for name in ["timeout", "connect_timeout"] {
        for value in ["", "bad", "0", "-0", "-1", "NaN", "inf", "-inf", "1e100", "1e-100"] {
            let url = format!("memcache://localhost:11211?{name}={value}");
            assert!(matches!(Manager::new(url), Err(MemcacheError::BadURL(_))), "{name}={value}");
        }

        let url = format!("memcache://localhost:11211?{name}=bad&{name}=1");
        assert!(matches!(Manager::new(url), Err(MemcacheError::BadURL(_))));
    }
}

#[test]
fn empty_url_list_is_an_error() {
    assert!(matches!(Manager::new(Vec::<String>::new()), Err(MemcacheError::BadURL(_))));
}

#[test]
fn first_url_timeout_is_used() {
    for name in ["timeout", "connect_timeout"] {
        let url = format!("memcache://localhost:11211?{name}=1&{name}=bad");
        assert!(Manager::new(url.as_str()).is_ok(), "{url}");
    }
}

#[rocket::async_test]
async fn invalid_duration_is_a_custom_error() {
    let config = Figment::from(rocket::Config::debug_default())
        .merge(("databases.memcache", Config {
            url: "memcache://localhost:11211?timeout=NaN".into(),
            pool_size: 1,
            timeout: 1,
        }));
    let rocket = rocket::custom(config);
    let result = memcache::Client::pool("memcache", &rocket);
    assert!(matches!(result, Err(Error::Custom(MemcacheError::BadURL(_)))));
    let error = rocket.attach(MemcacheDb::fairing()).ignite().await.unwrap_err();
    assert!(matches!(error.kind(), rocket::error::ErrorKind::FailedFairings(_)));
}

#[test]
fn explicit_tcp_preserves_socket_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "memcache+tcp://{}?protocol=ascii&timeout=0.05&connect_timeout=1",
        listener.local_addr().unwrap()
    );
    let client = Manager::new(url).unwrap().connect().unwrap();
    let (done, finished) = mpsc::channel();
    let server = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(socket);
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "get slow\r\n" {
                let _ = finished.recv_timeout(Duration::from_secs(5));
                break;
            }

            assert_eq!(line, "version\r\n");
            reader.get_mut().write_all(b"VERSION 1.0\r\n").unwrap();
        }
    });

    let result = client.get::<String>("slow");
    let _ = done.send(());
    server.join().unwrap();
    assert!(matches!(result, Err(MemcacheError::IOError(error))
        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)));
}
