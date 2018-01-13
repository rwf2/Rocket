# Pastebin

To give you a taste of what a real Rocket application looks like, this section
of the guide is a tutorial on how to create a Pastebin application in Rocket. A
pastebin is a simple web application that allows users to upload a text document
and later retrieve it via a special URL. They're often used to share code
snippets, configuration files, and error logs. In this tutorial, we'll build a
simple pastebin service that allows users to upload a file from their terminal.
The service will respond back with a URL to the uploaded file.

## Finished Product

A souped-up, completed version of the application you're about to build is
deployed live at [paste.rs](https://paste.rs). Feel free to play with the
application to get a feel for how it works. For example, to upload a text
document named `test.txt`, you can do:

```sh
curl --data-binary @test.txt https://paste.rs/
# => https://paste.rs/IYu
```

The finished product is composed of the following routes:

  * index: **GET /** - returns a simple HTML page with instructions about how
    to use the service
  * upload: **POST /** - accepts raw data in the body of the request and
    responds with a URL of a page containing the body's content
  * retrieve: **GET /&lt;id>** - retrieves the content for the paste with id
    `<id>`

## Getting Started

Let's get started! First, create a fresh Cargo binary project named
`rocket-pastebin`:

```rust
cargo new --bin rocket-pastebin
cd rocket-pastebin
```

Then add the usual Rocket dependencies to the `Cargo.toml` file:

```toml
[dependencies]
rocket = "0.4.0-dev"
rocket_codegen = "0.4.0-dev"
```

And finally, create a skeleton Rocket application to work off of in
`src/main.rs`:

```rust
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

fn main() {
    rocket::ignite().launch();
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

The first route we'll create is the `index` route. This is the page users will
see when they first visit the service. As such, the route should field requests
of the form `GET /`. We declare the route and its handler by adding the `index`
function below to `src/main.rs`:

```rust
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
can read more about how Rocket formulates responses at the [API documentation
for the Responder
  trait](https://api.rocket.rs/rocket/response/trait.Responder.html).

Remember that routes first need to be mounted before Rocket dispatches requests
to them. To mount the `index` route, modify the main function so that it reads:

```rust
fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
```

You should now be able to `cargo run` the application and visit the root path
(`/`) to see the text being displayed.

## Uploading

The most complicated aspect of the pastebin, as you might imagine, is handling
upload requests. When a user attempts to upload a pastebin, our service needs to
generate a unique ID for the upload, read the data, write it out to a file or
database, and then return a URL with the ID. We'll take each of these one step
at a time, beginning with generating IDs.

### Unique IDs

Generating a unique and useful ID is an interesting topic, but it is outside the
scope of this tutorial. Instead, we simply provide the code for a `PasteID`
structure that represents a _probably_ unique ID. Read through the code, then
copy/paste it into a new file named `paste_id.rs` in the `src/` directory:

```rust
use std::fmt;
use std::borrow::Cow;

use rand::{self, Rng};

/// Table to retrieve base62 values from.
const BASE62: &'static [u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A _probably_ unique paste ID.
pub struct PasteID<'a>(Cow<'a, str>);

impl<'a> PasteID<'a> {
    /// Generate a _probably_ unique ID with `size` characters. For readability,
    /// the characters used are from the sets [0-9], [A-Z], [a-z]. The
    /// probability of a collision depends on the value of `size` and the number
    /// of IDs generated thus far.
    pub fn new(size: usize) -> PasteID<'static> {
        let mut id = String::with_capacity(size);
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            id.push(BASE62[rng.gen::<usize>() % 62] as char);
        }

        PasteID(Cow::Owned(id))
    }
}

impl<'a> fmt::Display for PasteID<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

Then, in `src/main.rs`, add the following after `extern crate rocket`:

```rust
extern crate rand;

mod paste_id;

use paste_id::PasteID;
```

Finally, add a dependency for the `rand` crate to the `Cargo.toml` file:

```toml
[dependencies]
# existing Rocket dependencies...
rand = "0.3"
```

Then, ensure that your application builds with the new code:

```sh
cargo build
```

You'll likely see many "unused" warnings for the new code we've added: that's
okay and expected. We'll be using the new code soon.

### Processing

Believe it or not, the hard part is done! (_whew!_).

To process the upload, we'll need a place to store the uploaded files. To
simplify things, we'll store the uploads in a directory named `upload/`. Create
an `upload` directory next to the `src` directory:

```sh
mkdir upload
```

For the `upload` route, we'll need to `use` a few items:

```rust
use std::io;
use std::path::Path;

use rocket::Data;
use rocket::http::RawStr;
```

The [Data](https://api.rocket.rs/rocket/data/struct.Data.html) structure is key
here: it represents an unopened stream to the incoming request body data. We'll
use it to efficiently stream the incoming request to a file.

### Upload Route

We're finally ready to write the `upload` route. Before we show you the code,
you should attempt to write the route yourself. Here's a hint: a possible route
and handler signature look like this:

```rust
#[post("/", data = "<paste>")]
fn upload(paste: Data) -> io::Result<String>
```

Your code should:

  1. Create a new `PasteID` of a length of your choosing.
  2. Construct a filename inside `upload/` given the `PasteID`.
  3. Stream the `Data` to the file with the constructed filename.
  4. Construct a URL given the `PasteID`.
  5. Return the URL to the client.

Here's our version (in `src/main.rs`):

```rust
#[post("/", data = "<paste>")]
fn upload(paste: Data) -> io::Result<String> {
    let id = PasteID::new(3);
    let filename = format!("upload/{id}", id = id);
    let url = format!("{host}/{id}\n", host = "http://localhost:8000", id = id);

    // Write the paste out to the file and return the URL.
    paste.stream_to_file(Path::new(&filename))?;
    Ok(url)
}
```

Make sure that the route is mounted at the root path:

```rust
fn main() {
    rocket::ignite().mount("/", routes![index, upload]).launch();
}
```

Test that your route works via `cargo run`. From a separate terminal, upload a
file using `curl`. Then verify that the file was saved to the `upload` directory
with the correct ID:

```sh
# in the project root
cargo run

# in a seperate terminal
echo "Hello, world." | curl --data-binary @- http://localhost:8000
# => http://localhost:8000/eGs

# back to the terminal running the pastebin
<ctrl-c>     # kill running process
ls upload    # ensure the upload is there
cat upload/* # ensure that contents are correct
```

Note that since we haven't created a `GET /<id>` route, visting the returned URL
will result in a **404**. We'll fix that now.

## Retrieving Pastes

The final step is to create the `retrieve` route which, given an `<id>`, will
return the corresponding paste if it exists.

Here's a first take at implementing the `retrieve` route. The route below takes
in an `<id>` as a dynamic path element. The handler uses the `id` to construct a
path to the paste inside `upload/`, and then attempts to open the file at that
path, optionally returning the `File` if it exists. Rocket treats a `None`
[Responder](https://api.rocket.rs/rocket/response/trait.Responder.html#provided-implementations)
as a **404** error, which is exactly what we want to return when the requested
paste doesn't exist.

```rust
use std::fs::File;
use rocket::http::RawStr;

#[get("/<id>")]
fn retrieve(id: &RawStr) -> Option<File> {
    let filename = format!("upload/{id}", id = id);
    File::open(&filename).ok()
}
```

Unfortunately, there's a problem with this code. Can you spot the issue? The
[`RawStr`](https://api.rocket.rs/rocket/http/struct.RawStr.html) type should tip
you off!

The issue is that the _user_ controls the value of `id`, and as a result, can
coerce the service into opening files inside `upload/` that aren't meant to be
opened. For instance, imagine that you later decide that a special file
`upload/_credentials.txt` will store some important, private information. If the
user issues a `GET` request to `/_credentials.txt`, the server will read and
return the `upload/_credentials.txt` file, leaking the sensitive information.
This is a big problem; it's known as the [full path disclosure
attack](https://www.owasp.org/index.php/Full_Path_Disclosure), and Rocket
provides the tools to prevent this and other kinds of attacks from happening.

To prevent the attack, we need to _validate_ `id` before we use it. Since the
`id` is a dynamic parameter, we can use Rocket's
[FromParam](https://api.rocket.rs/rocket/request/trait.FromParam.html) trait to
implement the validation and ensure that the `id` is a valid `PasteID` before
using it. We do this by implementing `FromParam` for `PasteID` in
`src/paste_id.rs`, as below:

```rust
use rocket::request::FromParam;

/// Returns `true` if `id` is a valid paste ID and `false` otherwise.
fn valid_id(id: &str) -> bool {
    id.chars().all(|c| {
        (c >= 'a' && c <= 'z')
            || (c >= 'A' && c <= 'Z')
            || (c >= '0' && c <= '9')
    })
}

/// Returns an instance of `PasteID` if the path segment is a valid ID.
/// Otherwise returns the invalid ID as the `Err` value.
impl<'a> FromParam<'a> for PasteID<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<PasteID<'a>, &'a RawStr> {
        match valid_id(param) {
            true => Ok(PasteID(Cow::Borrowed(param))),
            false => Err(param)
        }
    }
}
```

Then, we simply need to change the type of `id` in the handler to `PasteID`.
Rocket will then ensure that `<id>` represents a valid `PasteID` before calling
the `retrieve` route, preventing attacks on the `retrieve` route:

```rust
#[get("/<id>")]
fn retrieve(id: PasteID) -> Option<File> {
    let filename = format!("upload/{id}", id = id);
    File::open(&filename).ok()
}
```

Note that our `valid_id` function is simplistic and could be improved by, for
example, checking that the length of the `id` is within some known bound or
potentially blacklisting sensitive files as needed.

The wonderful thing about using `FromParam` and other Rocket traits is that they
centralize policies. For instance, here, we've centralized the policy for valid
`PasteID`s in dynamic parameters. At any point in the future, if other routes
are added that require a `PasteID`, no further work has to be done: simply use
the type in the signature and Rocket takes care of the rest.

## Conclusion

That's it! Ensure that all of your routes are mounted and test your application.
You've now written a simple (~75 line!) pastebin in Rocket! There are many
potential improvements to this small application, and we encourage you to work
through some of them to get a better feel for Rocket. Here are some ideas:

  * Add a web form to the `index` where users can manually input new pastes.
    Accept the form at `POST /`. Use `format` and/or `rank` to specify which of
    the two `POST /` routes should be called.
  * Support **deletion** of pastes by adding a new `DELETE /<id>` route. Use
    `PasteID` to validate `<id>`.
  * **Limit the upload** to a maximum size. If the upload exceeds that size,
    return a **206** partial status code. Otherwise, return a **201** created
    status code.
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
  * Use the [`local` module](https://api.rocket.rs/rocket/local/) to write
    unit tests for your pastebin.
  * Dispatch a thread before `launch`ing Rocket in `main` that periodically
    cleans up idling old pastes in `upload/`.

You can find the full source code for the [completed pastebin tutorial on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.4.0-dev/examples/pastebin).
