use rocket::data::TempFileRead;
use std::io::Cursor;
use tokio::{fs::File, io::AsyncReadExt};

#[tokio::test]
async fn read_temp_file() {
    let mut tfr = TempFileRead::File(File::open("Cargo.toml").await.unwrap());
    let mut content = Vec::new();
    tfr.read_to_end(&mut content).await.unwrap();
    assert!(content.starts_with("[package]".as_bytes()));
}

#[tokio::test]
async fn read_temp_buffer() {
    let mut tfr = TempFileRead::Buffered(Cursor::new("Hello!World!".as_bytes()));
    let mut content = Vec::new();
    tfr.read_to_end(&mut content).await.unwrap();
    assert_eq!(&content, "Hello!World!".as_bytes());
}

#[tokio::test]
async fn read_temp_buffer_owned() {
    let mut tfr = TempFileRead::BufferedOwned(Cursor::new("Hello!World!".as_bytes().to_vec()));
    let mut content = Vec::new();
    tfr.read_to_end(&mut content).await.unwrap();
    assert_eq!(&content, "Hello!World!".as_bytes());
}

#[tokio::test]
async fn read_temp_buffer_owned_position() {
    let mut tfr = TempFileRead::Buffered(Cursor::new("Hello!World!".as_bytes()));
    let mut start = [0; 6];
    tfr.read_exact(&mut start).await.unwrap();
    assert_eq!(&start, "Hello!".as_bytes());

    let mut tfr = tfr.into_owned();
    let mut end = Vec::new();
    tfr.read_to_end(&mut end).await.unwrap();
    assert_eq!(&end, "World!".as_bytes());
}
