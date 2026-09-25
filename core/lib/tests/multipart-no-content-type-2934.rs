#[macro_use] extern crate rocket;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rocket::data::{Capped, Limits, ToByteUnit};
use rocket::form::{self, Contextual, DataField, Form, FromFormField, Lenient, Strict, ValueField};
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
}

#[post("/", data = "<upload>")]
async fn upload(mut upload: Form<Upload<'_>>) -> Vec<u8> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("received");
    upload.file.persist_to(&path).await.unwrap();
    std::fs::read(path).unwrap()
}

fn check_upload(filename: bool, content_type: bool) {
    let bytes: Vec<u8> = (0..=255).collect();
    check_bytes(filename, content_type, &bytes);
}

fn check_bytes(filename: bool, content_type: bool, bytes: &[u8]) {
    let client = Client::debug_with(routes![upload]).unwrap();
    let mut body = b"--boundary\r\nContent-Disposition: form-data; name=\"file\"".to_vec();
    if filename {
        body.extend_from_slice(b"; filename=\"test.bin\"");
    }

    body.extend_from_slice(b"\r\n");
    if content_type {
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
    }

    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--boundary--\r\n");
    let response = client.post("/")
        .header(ContentType::new("multipart", "form-data").with_params([("boundary", "boundary")]))
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_bytes().unwrap(), bytes);
}

#[test]
fn filename_without_content_type() {
    check_upload(true, false);
}

#[test]
fn no_filename_or_content_type() {
    check_upload(false, false);
}

#[test]
fn filename_with_content_type() {
    check_upload(true, true);
}

#[test]
fn content_type_without_filename() {
    check_upload(false, true);
}

#[test]
fn utf8_bom_without_content_type() {
    check_bytes(false, false, b"\xef\xbb\xbfhello");
}

#[test]
fn utf16_bom_without_content_type() {
    check_bytes(false, false, b"\xff\xfeh\0i\0");
}

#[derive(FromForm)]
struct TextFields<'r> {
    number: u32,
    enabled: bool,
    text: &'r str,
}

#[post("/text", data = "<fields>")]
fn text(fields: Form<TextFields<'_>>) -> String {
    format!("{}:{}:{}", fields.number, fields.enabled, fields.text)
}

#[test]
fn ordinary_headerless_text_fields() {
    let client = Client::debug_with(routes![text]).unwrap();
    let body = [
        "--boundary",
        "Content-Disposition: form-data; name=\"number\"",
        "",
        "42",
        "--boundary",
        "Content-Disposition: form-data; name=\"enabled\"",
        "",
        "yes",
        "--boundary",
        "Content-Disposition: form-data; name=\"text\"",
        "",
        "Grüße + %20",
        "--boundary--",
        "",
    ].join("\r\n");
    let response = client.post("/text")
        .header(ContentType::new("multipart", "form-data").with_params([("boundary", "boundary")]))
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "42:true:Grüße + %20");
}

fn multipart(fields: &[(&str, &[u8])], content_type: bool) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, bytes) in fields {
        body.extend_from_slice(format!("--boundary\r\n\
            Content-Disposition: form-data; name=\"{name}\"\r\n").as_bytes());
        if content_type {
            body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        }

        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(b"--boundary--\r\n");
    body
}

fn multipart_type() -> ContentType {
    ContentType::new("multipart", "form-data").with_params([("boundary", "boundary")])
}

#[test]
fn empty_and_text_uploads() {
    for bytes in [b"".as_slice(), b"plain text", "Grüße".as_bytes()] {
        check_bytes(false, false, bytes);
        check_bytes(true, false, bytes);
    }
}

#[derive(FromForm)]
struct Composed<'r> {
    files: Vec<TempFile<'r>>,
    map: HashMap<String, TempFile<'r>>,
    keys: BTreeMap<&'r [u8], &'r [u8]>,
    pair: (TempFile<'r>, &'r [u8]),
    optional: Option<TempFile<'r>>,
    result: form::Result<'r, TempFile<'r>>,
    arc: Arc<TempFile<'r>>,
    strict: Strict<TempFile<'r>>,
    lenient: Lenient<TempFile<'r>>,
    capped: Capped<TempFile<'r>>,
    capped_bytes: Capped<&'r [u8]>,
    range: std::ops::Range<&'r [u8]>,
    context: Contextual<'r, Upload<'r>>,
    #[field(name = uncased("renamed"))]
    alias: &'r [u8],
    decoded: &'r str,
}

#[post("/composed", data = "<form>")]
async fn composed(form: Form<Composed<'_>>) {
    use rocket::tokio::io::AsyncReadExt;

    let form = form.into_inner();
    let bytes = b"\xef\xbb\xbfhello";
    assert_eq!(form.decoded, "hello");
    assert_eq!(form.files.len(), 2);
    assert_eq!(form.pair.1, bytes);
    assert_eq!(form.keys.get(bytes.as_slice()), Some(&bytes.as_slice()));
    assert_eq!(form.range.start, bytes);
    assert_eq!(form.range.end, bytes);
    assert_eq!(form.alias, bytes);
    assert!(form.capped.is_complete());
    assert_eq!(form.capped.n.written, bytes.len() as u64);
    assert!(form.capped_bytes.is_complete());
    assert_eq!(form.capped_bytes.value, bytes);
    assert_eq!(form.context.context.field_value("context.file"), Some("hello"));

    let files = form.files.into_iter().chain([
        form.map.into_values().next().unwrap(),
        form.pair.0,
        form.optional.unwrap(),
        form.result.unwrap(),
        Arc::try_unwrap(form.arc).unwrap(),
        form.strict.into_inner(),
        form.lenient.into_inner(),
        form.capped.value,
        form.context.value.unwrap().file,
    ]);

    // Read after all later parts have been parsed to exercise borrowed storage.
    for file in files {
        assert!(file.path().is_none());
        assert!(file.content_type().is_none());
        let mut actual = Vec::new();
        file.open().await.unwrap().read_to_end(&mut actual).await.unwrap();
        assert_eq!(actual, bytes);
    }
}

#[test]
fn nested_forms_preserve_bytes_and_decoded_text() {
    let client = Client::debug_with(routes![composed]).unwrap();
    let names = [
        "decoded", "files[0]", "files[1]", "map[item]", "keys[k:0]", "keys[v:0]",
        "pair.0", "pair.1", "optional", "result", "arc", "strict", "lenient", "capped",
        "capped_bytes", "range.start", "range.end", "context.file", "ReNaMeD",
    ];
    let fields: Vec<_> = names.iter()
        .map(|name| (*name, b"\xef\xbb\xbfhello".as_slice())).collect();
    let response = client.post("/composed").header(multipart_type())
        .body(multipart(&fields, false)).dispatch();
    assert_eq!(response.status(), Status::Ok);
}

struct LegacyField<'r>(&'r str);

impl<'r> FromFormField<'r> for LegacyField<'r> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        Ok(Self(field.value))
    }
}

struct LegacyForm<'r>(Vec<&'r str>);

#[rocket::async_trait]
impl<'r> form::FromForm<'r> for LegacyForm<'r> {
    type Context = Vec<&'r str>;

    fn init(_: form::Options) -> Self::Context {
        Vec::new()
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
        ctxt.push(field.value);
    }

    async fn push_data(_: &mut Self::Context, _: DataField<'r, '_>) {
        panic!("a multipart value must not become a data field");
    }

    fn finalize(ctxt: Self::Context) -> form::Result<'r, Self> {
        Ok(Self(ctxt))
    }
}

#[post("/legacy", data = "<form>")]
fn legacy(form: Form<LegacyForm<'_>>) -> String {
    assert_eq!(form.0.len(), 1);
    form.0[0].to_owned()
}

#[post("/legacy-field", data = "<form>")]
fn legacy_field(form: Form<LegacyField<'_>>) -> String {
    form.0.to_owned()
}

#[test]
fn existing_text_callbacks_keep_decoding_behavior() {
    let client = Client::debug_with(routes![legacy, legacy_field]).unwrap();
    for (bytes, decoded) in [
        (b"\xff".as_slice(), "\u{fffd}"),
        (b"\xef\xbb\xbfhello", "hello"),
        (b"\xff\xfeh\0i\0", "hi"),
        (b"a+%20", "a+%20"),
    ] {
        for path in ["/legacy", "/legacy-field"] {
            let response = client.post(path).header(multipart_type())
                .body(multipart(&[("value", bytes)], false)).dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string().unwrap(), decoded);
        }
    }

    let field = ValueField { name: form::name::NameView::new("file"), value: "manual" };
    assert_eq!(Form::<&[u8]>::parse_iter([field]).unwrap(), b"manual");
    let response = client.post("/legacy").header(ContentType::Form)
        .body("value=a+%20").dispatch();
    assert_eq!(response.into_string().unwrap(), "a  ");
}

#[post("/strict", data = "<form>")]
fn strict(form: Form<Strict<Upload<'_>>>) -> String {
    form.file.len().to_string()
}

#[post("/bytes", data = "<form>")]
fn bytes(form: Form<&[u8]>) -> Vec<u8> {
    form.into_inner().to_vec()
}

#[post("/string", data = "<form>")]
fn string(form: Form<&str>) -> String {
    form.into_inner().to_owned()
}

#[post("/capped", data = "<form>")]
fn capped(form: Form<Capped<TempFile<'_>>>) -> String {
    format!("{}:{}", form.n.written, form.is_complete())
}

#[test]
fn limits_duplicates_and_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let limits = Limits::default().limit("file", 2.bytes())
        .limit("string", 2.bytes()).limit("bytes", 2.bytes());
    let figment = rocket::Config::figment()
        .merge(("limits", limits)).merge(("temp_dir", directory.path()));
    let client = Client::debug(rocket::custom(figment)
        .mount("/", routes![strict, bytes, string, capped])).unwrap();

    for path in ["/strict", "/bytes", "/string"] {
        for content_type in [false, true] {
            let response = client.post(path).header(multipart_type())
                .body(multipart(&[("file", b"hello")], content_type)).dispatch();
            let expected = if content_type { Status::PayloadTooLarge } else { Status::Ok };
            assert_eq!(response.status(), expected, "{path}, content_type={content_type}");
        }
    }

    for (content_type, expected) in [(false, "5:true"), (true, "2:false")] {
        let response = client.post("/capped").header(multipart_type())
            .body(multipart(&[("file", b"hello")], content_type)).dispatch();
        assert_eq!(response.into_string().unwrap(), expected);
    }

    let body = multipart(&[("file", b"a"), ("file", b"b")], false);
    let response = client.post("/strict").header(multipart_type()).body(body).dispatch();
    assert_eq!(response.status(), Status::UnprocessableEntity);
    // Explicit data parts create files even when parsing fails or is capped.
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn whole_form_limit_and_malformed_body() {
    let payload: Vec<u8> = (0..=255).collect();
    let body = multipart(&[("file", &payload)], false);
    // An unread required field adds a Missing error, whose 422 status takes
    // precedence over the parser's length error.
    let cases = [(body.len(), Status::Ok), (body.len() - 1, Status::UnprocessableEntity)];
    for (limit, status) in cases {
        let limits = Limits::default().limit("data-form", limit.bytes());
        let figment = rocket::Config::figment().merge(("limits", limits));
        let client = Client::debug(rocket::custom(figment).mount("/", routes![upload])).unwrap();
        let response = client.post("/").header(multipart_type()).body(body.clone()).dispatch();
        assert_eq!(response.status(), status);
        if status == Status::Ok {
            assert_eq!(response.into_bytes().unwrap(), payload);
        }
    }

    let client = Client::debug_with(routes![upload]).unwrap();
    let response = client.post("/").header(multipart_type())
        .body(b"--boundary\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\n\xff")
        .dispatch();
    assert_eq!(response.status(), Status::UnprocessableEntity);
}
