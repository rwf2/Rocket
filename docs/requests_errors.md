## Errors
Responders need not always generate a response. Instead, they can return an
Err with a given status code. When this happens, Rocket forwards the request
to the error catcher for the given status code. If none exists, which can
only happen when using custom status codes, Rocket uses the 500 error catcher.

### Result
`Result` is one of the most commonly used responders. Returning a `Result` means
one of two things. If the error type implements `Responder`, the `Ok` or `Err`
value will be used, whichever the variant is. If the error type does not
implement `Responder`, the error is printed to the console, and the request is
forwarded to the 500 error catcher.

### Option
`Option` is another commonly used responder. If the `Option` is `Some`, the wrapped
responder is used to respond to the client. Otherwise, the request is
forwarded to the 404 error catcher.

### Failure
While not encouraged, you can also forward a request to a catcher manually by
using the Failure type. For instance, to forward to the catcher for 406 Not
Acceptable, you would write:

```rust
#[get("/")]
fn just_fail() -> Failure {
    Failure(Status::NotAcceptable)
}
```