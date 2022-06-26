# Pastebin Tutorial

This section of the guide is a tutorial intended to demonstrate how real-world
Rocket applications are crafted. We'll build a simple pastebin service that
allows users to upload a file from any HTTP client, including `curl`. The
service will respond back with a URL to the uploaded file.

! note: What's a pastebin?

  A pastebin is a simple web application that allows users to upload a document
  and later retrieve it via a special URL. They're often used to share code
  snippets, configuration files, and error logs.

## Finished Product

A souped-up, completed version of the application you're about to build is
deployed live at [paste.rs](https://paste.rs). Feel free to play with the
application to get a feel for how it works. For example, to upload a text
document named `test.txt`, you can run:

```sh
curl --data-binary @test.txt https://paste.rs/
# => https://paste.rs/IYu
```

The finished product is composed of the following routes:

  * `index` - `#[get("/")]`

    returns a simple HTML page with instructions about how to use the service

  * `upload` - `#[post("/")]`

    accepts raw data in the body of the request and responds with a URL of a
    page containing the body's content

  * `retrieve` - `#[get("/<id>")]`

    retrieves the content for the paste with id `<id>`

## Getting Started

Let's get started! First, create a fresh Cargo binary project named
`rocket-pastebin`:

```sh
cargo new --bin rocket-pastebin
cd rocket-pastebin
```

Then add the usual Rocket dependencies to the `Cargo.toml` file:

```toml
[dependencies]
rocket = "0.5.0-rc.2"
```

And finally, create a skeleton Rocket application to work off of in
`src/main.rs`:

```rust
#[macro_use] extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build()
}
```

Ensure everything works by running the application:

```sh
cargo run
```

At this point, we haven't declared any routes or handlers, so visiting any page
will result in Rocket returning a **404** error. Throughout the rest of the
tutorial, we'll create the three routes and accompanying handlers.

## Index

The first route we'll create is `index`. This is the page users will see when
they first visit the service. As such, the route should handle `GET /`. We
declare the route and its handler by adding the `index` function below to
`src/main.rs`:

```rust
# #[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "
    USAGE

      POST /

          accepts raw data in the body of the request and responds with a URL of
          a page containing the body's content

      GET /<id>

          retrieves the content for the paste with id `<id>`
    "
}
```

This declares the `index` route for requests to `GET /` as returning a static
string with the specified contents. Rocket will take the string and return it as
the body of a fully formed HTTP response with `Content-Type: text/plain`. You
can read more about how Rocket formulates responses in the [responses section]
of the guide or at the [API documentation for the Responder
trait](@api/rocket/response/trait.Responder.html).

[responses section]: ../responses

Remember that routes first need to be mounted before Rocket dispatches requests
to them. To mount the `index` route, modify the main function so that it reads:

```rust
# #[macro_use] extern crate rocket;
# #[get("/")] fn index() { }

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
```

You should now be able to `cargo run` the application and visit the root path
(`/`) to see the text.

## Design

Before we continue, we'll need to make a few design decisions.

  * **Where should pastes be stored?**

    To keep things simple, we'll store uploaded pastes on the file system inside
    of an `upload/` directory. Let's create that directory next to `src/` in our
    project now:

    ```sh
    mkdir upload
    ```

    Our project tree now looks like:

    ```sh
    .
    ├── Cargo.toml
    ├── src
    │   └── main.rs
    └── upload
    ```

  * **What should we name the uploaded paste files?**

    Similarly, we'll keep things simple by naming paste files a string of random
    but readable characters. We'll call this random string the paste's "ID". To
    represent, generate, and store the ID, we'll create a `PasteId` structure in
    a new module file named `paste_id.rs` with the following contents:

    ```rust
    use std::borrow::Cow;
    use std::path::{Path, PathBuf};

    use rand::{self, Rng};

    /// A _probably_ unique paste ID.
    pub struct PasteId<'a>(Cow<'a, str>);

    impl PasteId<'_> {
        /// Generate a _probably_ unique ID with `size` characters. For readability,
        /// the characters used are from the sets [0-9], [A-Z], [a-z]. The
        /// probability of a collision depends on the value of `size` and the number
        /// of IDs generated thus far.
        pub fn new(size: usize) -> PasteId<'static> {
            const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

            let mut id = String::with_capacity(size);
            let mut rng = rand::thread_rng();
            for _ in 0..size {
                id.push(BASE62[rng.gen::<usize>() % 62] as char);
            }

            PasteId(Cow::Owned(id))
        }

        /// Returns the path to the paste in `upload/` corresponding to this ID.
        pub fn file_path(&self) -> PathBuf {
            let root = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
            Path::new(root).join(self.0.as_ref())
        }
    }
    ```

    We've given you the ID and path generation code for free. Our project tree
    now looks like:

    ```sh
    .
    ├── Cargo.toml
    ├── src
    │   ├── main.rs
    │   └── paste_id.rs # new! contains `PasteId`
    └── upload
    ```

    We'll import the new module and struct in `src/main.rs`, after the `extern
    crate rocket`:

    ```rust
    # /*
    mod paste_id;
    # */ mod paste_id { pub struct PasteId; }

    use paste_id::PasteId;
    ```

    You'll notice that our code to generate paste IDs uses the `rand` crate, so
    we'll need to add it as a dependency in our `Cargo.toml` file:

    ```toml
    [dependencies]
    ## existing Rocket dependencies...
    rand = "0.8"
    ```

    Ensure that your application builds with the new code:

    ```sh
    cargo build
    ```

    You'll likely see many "unused" warnings for the new code we've added: that's
    okay and expected. We'll be using the new code soon.

With these design decisions made, we're ready to continue writing our
application.

## Retrieving Pastes

We'll proceed with a `retrieve` route which, given an `<id>`, will return the
corresponding paste if it exists or otherwise **404**. As we now know, that
means we'll be reading the contents of the file corresponding to `<id>` in the
`upload/` directory and return them to the user.

Here's a first take at implementing the `retrieve` route. The route below takes
in an `<id>` as a dynamic path element. The handler uses the `id` to construct a
path to the paste inside `upload/`, and then attempts to open the file at that
path, optionally returning the `File` if it exists. Rocket treats a `None`
[Responder](@api/rocket/response/trait.Responder.html#provided-implementations)
as a **404** error, which is exactly what we want to return when the requested
paste doesn't exist.

```rust
# #[macro_use] extern crate rocket;

use std::path::Path;
use rocket::tokio::fs::File;

#[get("/<id>")]
async fn retrieve(id: &str) -> Option<File> {
    let upload_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let filename = Path::new(upload_dir).join(id);
    File::open(&filename).await.ok()
}
```

Make sure that the route is mounted at the root path:

```rust
# #[macro_use] extern crate rocket;

# #[get("/")] fn index() {}
# #[get("/<id>")] fn retrieve(id: String) {}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, retrieve])
}
```

Give it a try! Create some fake pastes in the `upload/` directory, run the
application, and try to retrieve them by visiting the corresponding URL.

### A Problem

Unfortunately, there's a problem with this code. Can you spot the issue? The
`&str` type in `retrieve` should tip you off! We've crafted a wonderful type to
represent paste IDs but have ignored it!

The issue is that the _user_ controls the value of `id`, and as a result, can
coerce the service into opening files inside `upload/` that aren't meant to be
opened. For instance, imagine that you later decide that a special file
`upload/_credentials.txt` will store some important, private information. If the
user issues a `GET` request to `/_credentials.txt`, the server will read and
return the `upload/_credentials.txt` file, leaking the sensitive information.
This is a big problem; it's known as the [full path disclosure
attack](https://www.owasp.org/index.php/Full_Path_Disclosure), and Rocket
provides the tools to prevent this and other kinds of attacks from happening.

### The Solution

To prevent the attack, we need to _validate_ `id` before we use it. We do so by
using a type more specific than `&str` to represent IDs and then asking Rocket
to validate the untrusted `id` input as that type. If validation fails, Rocket
will take care to not call our routes with bad input.

Typed validation for dynamic paramters like `id` is implemented via the
[`FromParam`] trait. Rocket uses `FromParam` to automatically validate and parse
dynamic path parameters like `id`. We already have a type that represents valid
paste IDs, `PasteId`, so we'll simply need to implement `FromParam` for
`PasteId`.

Here's the `FromParam` implementation for `PasteId` in `src/paste_id.rs`:

[`FromParam`]: @api/rocket/request/trait.FromParam.html

```rust
use rocket::request::FromParam;
# use std::borrow::Cow;
# pub struct PasteId<'a>(Cow<'a, str>);

/// Returns an instance of `PasteId` if the path segment is a valid ID.
/// Otherwise returns the invalid ID as the `Err` value.
impl<'a> FromParam<'a> for PasteId<'a> {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.chars().all(|c| c.is_ascii_alphanumeric())
            .then(|| PasteId(param.into()))
            .ok_or(param)
    }
}
```

! note: This implementation, while secure, could be improved.

  Our `from_param` function is simplistic and could be improved by, for example,
  checking that the length of the `id` is within some known bound, introducing
  stricter character checks, checking for the existing of a paste file, and/or
  potentially blacklisting sensitive files as needed.

Given this implementation, we can change the type of `id` in `retrieve` to
`PasteId`. Rocket will then ensure that `<id>` represents a valid `PasteId`
before calling the `retrieve` route, preventing the previous attack entirely:

```rust
# #[macro_use] extern crate rocket;

use rocket::tokio::fs::File;
# use std::borrow::Cow;
# use std::path::PathBuf;
# use rocket::request::FromParam;
# pub struct PasteId<'a>(Cow<'a, str>);
# impl PasteId<'_> {
#     pub fn new(size: usize) -> PasteId<'static> { todo!() }
#     pub fn file_path(&self) -> PathBuf { todo!() }
# }
# impl<'a> FromParam<'a> for PasteId<'a> {
#     type Error = &'a str;
#     fn from_param(param: &'a str) -> Result<Self, Self::Error> { todo!() }
# }

#[get("/<id>")]
async fn retrieve(id: PasteId<'_>) -> Option<File> {
    File::open(id.file_path()).await.ok()
}
```

Notice how much nicer this implementation is! And this time, it's secure.

The wonderful thing about using `FromParam` and other Rocket traits is that they
centralize policies. For instance, here, we've centralized the policy for valid
`PasteId`s in dynamic parameters. At any point in the future, if other routes
are added that require a `PasteId`, no further work has to be done: simply use
the type in the signature and Rocket takes care of the rest.


## Uploading

Now that we can retrieve pastes safely, it's time to actually store them. We'll
write an `upload` route that, according to our design, takes a paste's contents
and writes them to a file with a randomly generated ID inside of the `upload/`
directory. It'll return a URL to the client for the paste corresponding to the
`retrieve` route we just route.

### Streaming Data

To stream the incoming paste data to a file, we'll make use of [`Data`], a [data
guard] that represents an unopened stream to the incoming request body data.
Before we show you the code, you should attempt to write the route yourself.
Here's a hint: one possible route and handler signature look like this:

```rust
# #[macro_use] extern crate rocket;
use rocket::Data;

#[post("/", data = "<paste>")]
async fn upload(paste: Data<'_>) -> std::io::Result<String> {
    /* .. */
    # Ok("".into())
}
```

[`Data`]: @api/rocket/data/struct.Data.html
[data guard]: ../requests/#body-data

Your code should:

  1. Create a new `PasteId` of a length of your choosing.
  2. Construct a path to the `PasteId` inside of `upload/`.
  3. Stream the `Data` to the file at the constructed path.
  4. Construct a URL for the `PasteId`.
  5. Return the URL to the client.

### Solution

Here's our version:

```rust
# #[macro_use] extern crate rocket;

// We derive `UriDisplayPath` for `PasteId` in `paste_id.rs`:
# use std::borrow::Cow;
# use std::path::{Path, PathBuf};
# use rocket::request::FromParam;

#[derive(UriDisplayPath)]
pub struct PasteId<'a>(Cow<'a, str>);

# impl PasteId<'_> {
#     pub fn new(size: usize) -> PasteId<'static> { todo!() }
#     pub fn file_path(&self) -> PathBuf { todo!() }
# }
#
# impl<'a> FromParam<'a> for PasteId<'a> {
#     type Error = &'a str;
#     fn from_param(param: &'a str) -> Result<Self, Self::Error> { todo!() }
# }
// We implement the `upload` route in `main.rs`:

use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Absolute;
# use rocket::tokio::fs::File;

// In a real application, these would be retrieved dynamically from a config.
const ID_LENGTH: usize = 3;
const HOST: Absolute<'static> = uri!("http://localhost:8000");
# #[get("/")] fn index() -> &'static str { "" }
# #[get("/<id>")] fn retrieve(id: PasteId<'_>) -> Option<File> { todo!() }

#[post("/", data = "<paste>")]
async fn upload(paste: Data<'_>) -> std::io::Result<String> {
    let id = PasteId::new(ID_LENGTH);
    paste.open(128.kibibytes()).into_file(id.file_path()).await?;
    Ok(uri!(HOST, retrieve(id)).to_string())
}
```

We note the following Rocket APIs being used in our implementation:

  * The [`kibibytes()`] method, which comes from the [`ToByteUnit`] trait.
  * [`Data::open()`] to open [`Data`] as a [`DataStream`].
  * [`DataStream::into_file()`] for writing the data stream into a file.
  * The [`UriDisplayPath`] derive, allowing `PasteId` to be used in [`uri!`].
  * The [`uri!`] macro to crate type-safe, URL-safe URIs.

[`Data::open()`]: @api/rocket/data/struct.Data.html#method.open
[`Data`]: @api/rocket/data/struct.Data.html
[`DataStream`]: @api/rocket/data/struct.DataStream.html
[`DataStream::into_file()`]: @api/rocket/data/struct.DataStream.html#method.into_file
[`uri!`]: @api/rocket/macro.uri.html
[`kibibytes()`]: @api/rocket/data/trait.ToByteUnit.html#tymethod.kibibytes
[`ToByteUnit`]: @api/rocket/data/trait.ToByteUnit.html
[`UriDisplayPath`]: @api/rocket/derive.UriDisplayPath.html

Ensure that the route is mounted at the root path:

```rust
# #[macro_use] extern crate rocket;

# #[get("/")] fn index() {}
# #[get("/<id>")] fn retrieve(id: &str) {}
# #[post("/")] fn upload() {}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, retrieve, upload])
}
```

Test that your route works via `cargo run`. From a separate terminal, upload a
file using `curl` then retrieve the paste using the returned URL.

```sh
## in the project root
cargo run

## in a separate terminal
echo "Hello, Rocket!" | curl --data-binary @- http://localhost:8000
## => http://localhost:8000/eGs

## confirm we can retrieve the paste (replace with URL from above)
curl http://localhost:8000/eGs

## we can check the contents of `upload/` as well
<ctrl-c>     # kill running process
ls upload    # ensure the upload is there
cat upload/* # ensure that contents are correct
```

## Conclusion

That's it! Ensure that all of your routes are mounted and test your application.
You've now written a simple (~75 line!) pastebin in Rocket! There are many
potential improvements to this small application, and we encourage you to work
through some of them to get a better feel for Rocket. Here are some ideas:

  * Add a web form to the `index` where users can manually input new pastes.
    Accept the form at `POST /`. Use `format` and/or `rank` to specify which of
    the two `POST /` routes should be called.
  * Support **deletion** of pastes by adding a new `DELETE /<id>` route. Use
    `PasteId` to validate `<id>`.
  * Indicate **partial uploads** with a **206** partial status code. If the user
    uploads a paste that meets or exceeds the allowed limit, return a **206**
    partial status code. Otherwise, return a **201** created status code.
  * Set the `Content-Type` of the return value in `upload` and `retrieve` to
    `text/plain`.
  * **Return a unique "key"** after each upload and require that the key is
    present and matches when doing deletion. Use one of Rocket's core traits to
    do the key validation.
  * Add a `PUT /<id>` route that allows a user with the key for `<id>` to
    replace the existing paste, if any.
  * Add a new route, `GET /<id>/<lang>` that syntax highlights the paste with ID
    `<id>` for language `<lang>`. If `<lang>` is not a known language, do no
    highlighting. Possibly validate `<lang>` with `FromParam`.
  * Use the [`local` module](@api/rocket/local/) to write unit tests for your
    pastebin.
  * Dispatch a thread before `launch`ing Rocket in `main` that periodically
    cleans up idling old pastes in `upload/`.

You can find the full source code for the [completed pastebin tutorial on
GitHub](@example/pastebin).
