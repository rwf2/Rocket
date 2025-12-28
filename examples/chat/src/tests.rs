use std::ops::Range;

use rand::{rng, Rng};
use rand::distr::Alphanumeric;

use rocket::http::{ContentType, Status};
use rocket::http::uri::fmt::{UriDisplay, Query};
use rocket::local::asynchronous::{Client, LocalResponse};

use rocket::tokio::{sync, join};
use rocket::tokio::io::{BufReader, AsyncBufReadExt};
use rocket::serde::json;

use super::*;

async fn send_message<'c>(client: &'c Client, message: &Message) -> LocalResponse<'c> {
    client.post(uri!(post))
        .header(ContentType::Form)
        .body((message as &dyn UriDisplay<Query>).to_string())
        .dispatch()
        .await
}

fn gen_string(len: Range<usize>) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(rng().random_range(len))
        .map(char::from)
        .collect()
}

#[async_test]
async fn messages() {
    let client = Client::tracked(rocket()).await.unwrap();
    let start_barrier = sync::Barrier::new(2);

    let shutdown_message = Message {
        room: ":control".into(),
        username: ":control".into(),
        message: "shutdown".into(),
    };

    // Generate somewhere between 75 and 100 messages.
    let mut test_messages = vec![];
    for _ in 0..rng().random_range(75..100) {
        test_messages.push(Message {
            room: gen_string(10..30),
            username: gen_string(10..20),
            message: gen_string(10..100),
        })
    }

    let send_messages = async {
        // Wait for the other task to start listening.
        start_barrier.wait().await;

        // Send all of the messages.
        for message in &test_messages {
            send_message(&client, message).await;
        }

        // Send the special "shutdown" message.
        send_message(&client, &shutdown_message).await;
    };

    let receive_messages = async {
        let response = client.get(uri!(events)).dispatch().await;

        // We have the response stream. Let the receiver know to start sending.
        start_barrier.wait().await;

        let mut messages = vec![];
        let mut reader = BufReader::new(response).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if !line.starts_with("data:") {
                continue;
            }

            let data: Message = json::from_str(&line[5..]).expect("message JSON");
            if data == shutdown_message {
                // Test shutdown listening: this should end the stream.
                client.rocket().shutdown().notify();
                continue;
            }

            messages.push(data);
        }

        messages
    };

    let received_messages = join!(send_messages, receive_messages).1;
    assert!(test_messages.len() >= 75);
    assert_eq!(test_messages, received_messages);
}

#[async_test]
async fn bad_messages() {
    // Generate a bunch of bad messages.
    let mut bad_messages = vec![];
    for _ in 0..rng().random_range(75..100) {
        bad_messages.push(Message {
            room: gen_string(30..40),
            username: gen_string(20..30),
            message: gen_string(10..100),
        });
    }

    // Ensure they all result in a rejected request.
    let client = Client::tracked(rocket()).await.unwrap();
    for message in &bad_messages {
        let response = send_message(&client, message).await;
        assert_eq!(response.status(), Status::PayloadTooLarge);
    }
}
