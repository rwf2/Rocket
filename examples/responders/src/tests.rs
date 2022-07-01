use rocket::local::blocking::Client;
use rocket::http::Status;

/****************************** `File` Responder ******************************/

// We use a lock to synchronize between tests so FS operations don't race.
static FS_LOCK: parking_lot::Mutex<()> = parking_lot::const_mutex(());

#[test]
fn test_file() {
    const CONTENTS: &str = "big_file contents...not so big here";

    // Take the lock so we exclusively access the FS.
    let _lock = FS_LOCK.lock();

    // Create the 'big_file'
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.post(uri!(super::upload)).body(CONTENTS).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(response.into_string().unwrap().contains(&CONTENTS.len().to_string()));

    // Get the big file contents, hopefully.
    let res = client.get(uri!(super::file)).dispatch();
    assert_eq!(res.into_string(), Some(CONTENTS.into()));

    // Delete it.
    let response = client.delete(uri!(super::delete)).dispatch();
    assert_eq!(response.status(), Status::Ok);
}

/***************************** `Stream` Responder *****************************/

#[test]
fn test_many_his() {
    let client = Client::tracked(super::rocket()).unwrap();
    let res = client.get(uri!(super::many_his)).dispatch();

    // Check that we have exactly 100 `hi`s.
    let bytes = res.into_bytes().unwrap();
    assert_eq!(bytes.len(), 200);
    assert!(bytes.chunks(2).all(|b| b == b"hi"));
}

#[async_test]
async fn test_one_hi_per_second() {
    use rocket::local::asynchronous::Client;
    use rocket::tokio::time::{self, Instant, Duration};
    use rocket::tokio::{self, select};

    // Listen for 1 second at 1 `hi` per 250ms, see if we get ~4 `hi`'s, then
    // send a shutdown() signal, meaning we should get a `goodbye`.
    let client = Client::tracked(super::rocket()).await.unwrap();
    let response = client.get(uri!(super::one_hi_per_ms(250))).dispatch().await;
    let response = response.into_string();
    let timer = time::sleep(Duration::from_secs(1));

    tokio::pin!(timer, response);
    let start = Instant::now();
    let response = loop {
        select! {
            _ = &mut timer => {
                client.rocket().shutdown().notify();
                timer.as_mut().reset(Instant::now() + Duration::from_millis(100));
                if start.elapsed() > Duration::from_secs(2) {
                    panic!("responder did not terminate with shutdown");
                }
            }
            response = &mut response => break response.unwrap(),
        }
    };

    match &*response {
        "hihihigoodbye" | "hihihihigoodbye" | "hihihihihigoodbye" => { /* ok */ },
        s => panic!("unexpected response from infinite responder: {}", s)
    }
}

/***************************** `Redirect` Responder ***************************/

#[test]
fn test_redir_root() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get(uri!(super::redir_root)).dispatch();

    assert!(response.body().is_none());
    assert_eq!(response.status(), Status::SeeOther);
    for h in response.headers().iter() {
        match h.name.as_str() {
            "Location" => assert_eq!(h.value.as_ref(), &uri!(super::redir_login)),
            "Content-Length" => assert_eq!(h.value.parse::<i32>().unwrap(), 0),
            _ => { /* let these through */ }
        }
    }
}

#[test]
fn test_login() {
    let client = Client::tracked(super::rocket()).unwrap();
    let r = client.get(uri!(super::redir_login)).dispatch();
    assert_eq!(r.into_string().unwrap(), "Hi! Please log in before continuing.");

    for name in &["Bob", "Charley", "Joe Roger"] {
        let r = client.get(uri!(super::maybe_redir(name))).dispatch();
        assert_eq!(r.status(), Status::SeeOther);
    }

    let r = client.get(uri!(super::maybe_redir("Sergio"))).dispatch();
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.into_string().unwrap(), "Hello, Sergio!");
}

/***************************** `content` Responders ***************************/

use rocket::http::{Accept, ContentType};

#[test]
fn test_xml() {
    let client = Client::tracked(super::rocket()).unwrap();
    let r = client.get(uri!(super::xml)).header(Accept::XML).dispatch();
    assert_eq!(r.content_type().unwrap(), ContentType::XML);
    assert_eq!(r.into_string().unwrap(), "<payload>I'm here</payload>");

    // Purposefully use the "xml" URL to illustrate `format` handling.
    let r = client.get(uri!(super::xml)).header(Accept::JSON).dispatch();
    assert_eq!(r.content_type().unwrap(), ContentType::JSON);
    assert_eq!(r.into_string().unwrap(), r#"{ "payload": "I'm here" }"#);

    let r = client.get(uri!(super::xml)).header(Accept::CSV).dispatch();
    assert_eq!(r.status(), Status::NotFound);
    assert!(r.into_string().unwrap().contains("not supported"));

    let r = client.get("/content/i/dont/exist").header(Accept::HTML).dispatch();
    assert_eq!(r.content_type().unwrap(), ContentType::HTML);
    assert!(r.into_string().unwrap().contains("invalid path"));
}

/******************************* `Either` Responder ***************************/

#[test]
fn test_either() {
    let client = Client::tracked(super::rocket()).unwrap();
    let r = client.get(uri!(super::json_or_msgpack("json"))).dispatch();
    assert_eq!(r.content_type().unwrap(), ContentType::JSON);
    assert_eq!(r.into_string().unwrap(), "\"hi\"");

    let r = client.get(uri!(super::json_or_msgpack("msgpack"))).dispatch();
    assert_eq!(r.content_type().unwrap(), ContentType::MsgPack);
    assert_eq!(r.into_bytes().unwrap(), &[162, 104, 105]);
}

/******************************** Custom Responder ****************************/

use super::Kind;

#[test]
fn test_custom() {
    let client = Client::tracked(super::rocket()).unwrap();
    let r = client.get(uri!(super::custom(Some(Kind::String)))).dispatch();
    assert_eq!(r.into_string().unwrap(), "Hey, I'm some data.");

    let r = client.get(uri!(super::custom(Some(Kind::Bytes)))).dispatch();
    assert_eq!(r.into_string().unwrap(), "Hi");

    let r = client.get(uri!(super::custom(_))).dispatch();
    assert_eq!(r.status(), Status::Unauthorized);
    assert_eq!(r.content_type().unwrap(), ContentType::HTML);
    assert_eq!(r.into_string().unwrap(), "No no no!");

    // Take the lock so we exclusively access the FS.
    let _lock = FS_LOCK.lock();

    // Create the 'big_file'.
    const CONTENTS: &str = "custom file contents!";
    let response = client.post(uri!(super::upload)).body(CONTENTS).dispatch();
    assert_eq!(response.status(), Status::Ok);

    // Fetch it using `custom`.
    let r = client.get(uri!(super::custom(Some(Kind::File)))).dispatch();
    assert_eq!(r.into_string(), Some(CONTENTS.into()));

    // Delete it.
    let r = client.delete(uri!(super::delete)).dispatch();
    assert_eq!(r.status(), Status::Ok);
}
