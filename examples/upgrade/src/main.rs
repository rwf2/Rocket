#[macro_use] extern crate rocket;

use rocket::fs::{self, FileServer};
use rocket::futures::{SinkExt, StreamExt};

#[get("/echo?stream", rank = 1)]
fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
    ws::Stream! { ws =>
        for await message in ws {
            yield message?;
        }
    }
}

#[get("/echo?channel", rank = 2)]
fn echo_channel(ws: ws::WebSocket) -> ws::Channel<'static> {
    // This is entirely optional. Change default configuration.
    let ws = ws.config(ws::Config {
        // set max message size to 3MiB
        max_message_size: Some(3 << 20),
        ..Default::default()
    });

    ws.channel(move |mut stream| Box::pin(async move {
        while let Some(message) = stream.next().await {
            let _ = stream.send(message?).await;
        }

        Ok(())
    }))
}

#[get("/echo?raw", rank = 3)]
fn echo_raw(ws: ws::WebSocket) -> ws::Stream!['static] {
    ws.stream(|stream| stream)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![echo_channel, echo_stream, echo_raw])
        .mount("/", FileServer::from(fs::relative!("static")))
}
