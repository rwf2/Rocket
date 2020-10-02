#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::http::ContentType;
use rocket::response::Content;
use rocket::response::Responder;
use std::io::BufReader;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

#[get("/")]
fn index<'r>() -> impl Responder<'r> {
    Content(
        ContentType::HTML,
        r##"
<body>
<h1>Hi!</h1>

<div id="spong">nothing yet</div>

</body>
<script src="script.js"></script>
"##,
    )
}

#[get("/script.js")]
fn script<'r>() -> impl Responder<'r> {
    Content(
        ContentType::JavaScript,
        r##"
status_node = document.getElementById('spong');
status_node.innerHTML = 'js-done'

es = new EventSource("updates");
es.onmessage = function(event) {
  status_node.innerHTML = event.data;
}
"##,
    )
}

const BUF_SIZE : usize = 4096;

type TestCounter = BufReader<TestCounterInner>;
#[derive(Debug)]
struct TestCounterInner {
    next: usize,
}
impl Read for TestCounterInner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        sleep(Duration::from_millis(500));
        let data = format!("data: {}\n\n", self.next);
        self.next += 1;
        // `BufReader` won't call us unless its buffer is empty, and
        // then buf will be the whole of the buffer, ie of size
        // BUF_SIZE (due to the `with_capacity` call).  So `data` is
        // definitely going to fit.
        buf[0..data.len()].copy_from_slice(data.as_bytes());
        Ok(buf.len())
    }
}

#[get("/updates")]
fn updates<'x>() -> impl Responder<'x> {
    let tc = TestCounterInner { next: 0 };
    let tc = BufReader::with_capacity(BUF_SIZE, tc);
    let ch = rocket::response::Stream::from(tc);
    let ct = ContentType::parse_flexible("text/event-stream; charset=utf-8").unwrap();
    Content(ct, ch)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, script, updates,])
        .launch();
}
