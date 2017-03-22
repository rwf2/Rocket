#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::io;

use rocket::Data;
use rocket::response::content::Plain;

#[post("/upload", format = "text/plain", data = "<data>")]
fn upload(data: Data) -> io::Result<Plain<String>> {
    data.stream_to_file("/tmp/upload.txt").map(|n| Plain(n.to_string()))
}

#[get("/")]
fn index() -> &'static str {
    "Upload your text files by POSTing them to /upload."
}

fn main() {
    rocket::ignite().mount("/", routes![index, upload]).launch();
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::testing::MockRequest;
    use rocket::http::{Status, ContentType};
    use rocket::http::Method::Post;
    use rocket::response::NamedFile;
    use std::io::Read;

    #[test]
    fn test_raw_upload() {
        let text_to_upload = "Text to be uploaded!".to_string();

        // Upload the body
        let rocket =
            rocket::ignite().mount("/", routes![super::index, super::upload]);
        let mut request = MockRequest::new(Post, "/upload")
            .header(ContentType::Plain)
            .body(&text_to_upload);
        let response = request.dispatch_with(&rocket);
        assert_eq!(response.status(), Status::Ok);

        // Ensure we find the body in the /tmp/upload.txt file
        let mut found_text = String::new();
        let bytes_read = NamedFile::open("/tmp/upload.txt")
            .and_then(|mut f| f.read_to_string(&mut found_text));

        assert_eq!(bytes_read.expect("Unable to Read"), text_to_upload.capacity());
        assert_eq!(text_to_upload, found_text);
    }
}
