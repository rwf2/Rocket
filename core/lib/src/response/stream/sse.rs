use std::borrow::Cow;

use tokio::io::AsyncRead;
use tokio::time::Duration;
use futures::stream::{self, Stream, StreamExt};
use futures::future::ready;

use crate::request::Request;
use crate::response::{self, Response, Responder, stream::{ReaderStream, RawLinedEvent}};
use crate::http::ContentType;

/// A Server-Sent `Event` (SSE) in a Server-Sent [`struct@EventStream`].
///
/// A server-sent event is either a _field_ or a _comment_. Comments can be
/// constructed via [`Event::comment()`] while fields can be constructed via
/// [`Event::data()`], [`Event::json()`], and [`Event::retry()`].
///
/// ```rust
/// use rocket::tokio::time::Duration;
/// use rocket::response::stream::Event;
///
/// // A `data` event with message "Hello, SSE!".
/// let event = Event::data("Hello, SSE!");
///
/// // The same event but with event name of `hello`.
/// let event = Event::data("Hello, SSE!").event("hello");
///
/// // A `retry` event to set the client-side reconnection time.
/// let event = Event::retry(Duration::from_secs(5));
///
/// // An event with an attached comment, event name, and ID.
/// let event = Event::data("Hello, SSE!")
///     .with_comment("just a hello message")
///     .event("hello")
///     .id("1");
/// ```
///
/// We largely defer to [MDN's using server-sent events] documentation for
/// client-side details but reproduce, in our words, relevant server-side
/// documentation here.
///
/// [MDN's using server-sent events]:
/// https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
///
/// # Pitfalls
///
/// Server-Sent Events suffer from certain pitfalls. We encourage readers to
/// read through [pitfalls](struct@EventStream#pitfalls) before making use of
/// Rocket's SSE support.
///
/// # Comments
///
/// A server-sent _comment_, created via [`Event::comment()`], is an event that
/// appears only in the raw server-sent event data stream and is inaccessible by
/// most clients. This includes JavaScript's `EventSource`. As such, they serve
/// little utility beyond debugging a raw data stream and keeping a connection
/// alive. See [hearbeat](struct@EventStream#heartbeat) for information on
/// Rocket's `EventStream` keep-alive.
///
/// # Fields
///
/// A server-sent field can be one of four kinds:
///
///   * `retry`
///
///     A `retry` event, created via [`Event::retry()`], sets the reconnection
///     time on the client side. It is the duration the client will wait before
///     attempting to reconnect when a connection is lost. Most applications
///     will not need to set a `retry`, opting instead to use the
///     implementation's default or to reconnect manually on error.
///
///   * `id`
///
///     Sets the event id to associate all subsequent fields with. This value
///     cannot be retrieved directly via most clients, including JavaScript
///     `EventSource`. Instead, it is sent by the implementation on reconnection
///     via the `Last-Event-ID` header. An `id` can be attached to other fields
///     via the [`Event::id()`] builder method.
///
///   * `event`
///
///     Sets the event name to associate the next `data` field with. In
///     JavaScript's `EventSource`, this is the event to be listened for, which
///     defaults to `message`. An `event` can be attached to other fields via
///     the [`Event::event()`] builder method.
///
///   * `data`
///
///     Sends data to dispatch as an event at the client. In JavaScript's
///     `EventSource`, this (and only this) results in an event handler for
///     `event`, specified just prior, being triggered. A data field can be
///     created via the [`Event::data()`] or [`Event::json()`] constructors.
///
/// # Implementation Notes
///
/// A constructed `Event` _always_ emits its fields in the following order:
///
///   1. `comment`
///   2. `retry`
///   3. `id`
///   4. `event`
///   5. `data`
///
/// The `event` and `id` fields _cannot_ contain new lines or carriage returns.
/// Rocket's default implementation automatically converts new lines and
/// carriage returns in `event` and `id` fields to spaces.
///
/// The `data` and `comment` fields _cannot_ contain carriage returns. Rocket
/// converts the unencoded sequence `\r\n` and the isolated `\r` into a
/// protocol-level `\n`, that is, in such a way that they are interpreted as
/// `\n` at the client. For example, the raw message `foo\r\nbar\rbaz` is
/// received as `foo\nbar\nbaz` at the client-side. Encoded sequences, such as
/// those emitted by [`Event::json()`], have no such restrictions.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event {
    comment: Option<Cow<'static, str>>,
    retry: Option<Duration>,
    id: Option<Cow<'static, str>>,
    event: Option<Cow<'static, str>>,
    data: Option<Cow<'static, str>>,
}

impl Event {
    // We hide this since we never want to construct an `Event` with nothing.
    fn new() -> Self {
        Event { comment: None, retry: None, id: None, event: None, data: None, }
    }

    /// Creates a new `Event` with an empty data field.
    ///
    /// This is exactly equivalent to `Event::data("")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// let event = Event::empty();
    /// ```
    pub fn empty() -> Self {
        Event::data("")
    }

    /// Creates a new `Event` with a data field of `data` serialized as JSON.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::serde::Serialize;
    /// use rocket::response::stream::Event;
    ///
    /// #[derive(Serialize)]
    /// #[serde(crate = "rocket::serde")]
    /// struct MyData<'r> {
    ///     string: &'r str,
    ///     number: usize,
    /// }
    ///
    /// let data = MyData { string: "hello!", number: 10 };
    /// let event = Event::json(&data);
    /// ```
    #[cfg(feature = "json")]
    #[cfg_attr(nightly, doc(cfg(feature = "json")))]
    pub fn json<T: serde::Serialize>(data: &T) -> Self {
        let string = serde_json::to_string(data).unwrap_or_default();
        Self::data(string)
    }

    /// Creates a new `Event` with a data field containing the raw `data`.
    ///
    /// # Raw SSE is Lossy
    ///
    /// Unencoded carriage returns cannot be expressed in the protocol. Thus,
    /// any carriage returns in `data` will not appear at the client-side.
    /// Instead, the sequence `\r\n` and the isolated `\r` will each appear as
    /// `\n` at the client-side. For example, the message `foo\r\nbar\rbaz` is
    /// received as `foo\nbar\nbaz` at the client-side.
    ///
    /// See [pitfalls](struct@EventStream#pitfalls) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// // A `data` event with message "Hello, SSE!".
    /// let event = Event::data("Hello, SSE!");
    /// ```
    pub fn data<T: Into<Cow<'static, str>>>(data: T) -> Self {
        Self { data: Some(data.into()), ..Event::new() }
    }

    /// Creates a new comment `Event`.
    ///
    /// As with [`Event::data()`], unencoded carriage returns cannot be
    /// expressed in the protocol. Thus, any carriage returns in `data` will
    /// not appear at the client-side. For comments, this is generally not a
    /// concern as comments are discarded by client-side libraries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// let event = Event::comment("bet you'll never see me!");
    /// ```
    pub fn comment<T: Into<Cow<'static, str>>>(data: T) -> Self {
        Self { comment: Some(data.into()), ..Event::new() }
    }

    /// Creates a new retry `Event`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    /// use rocket::tokio::time::Duration;
    ///
    /// let event = Event::retry(Duration::from_millis(250));
    /// ```
    pub fn retry(period: Duration) -> Self {
        Self { retry: Some(period), ..Event::new() }
    }

    /// Sets the value of the 'event' (event type) field.
    ///
    /// Event names may not contain new lines `\n` or carriage returns `\r`. If
    /// `event` _does_ contain new lines or carriage returns, they are replaced
    /// with spaces (` `) before being sent to the client.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// // The event name is "start".
    /// let event = Event::data("hi").event("start");
    ///
    /// // The event name is "then end", with `\n` replaced with ` `.
    /// let event = Event::data("bye").event("then\nend");
    /// ```
    pub fn event<T: Into<Cow<'static, str>>>(mut self, event: T) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Sets the value of the 'id' (last event ID) field.
    ///
    /// Event IDs may not contain new lines `\n` or carriage returns `\r`. If
    /// `id` _does_ contain new lines or carriage returns, they are replaced
    /// with spaces (` `) before being sent to the client.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// // The event ID is "start".
    /// let event = Event::data("hi").id("start");
    ///
    /// // The event ID is "then end", with `\n` replaced with ` `.
    /// let event = Event::data("bye").id("then\nend");
    /// ```
    /// Sets the value of the 'id' field. It may not contain newlines.
    pub fn id<T: Into<Cow<'static, str>>>(mut self, id: T) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets or replaces the value of the `data` field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// // The data "hello" will be sent.
    /// let event = Event::data("hi").with_data("hello");
    ///
    /// // The two below are equivalent.
    /// let event = Event::comment("bye").with_data("goodbye");
    /// let event = Event::data("goodbyte").with_comment("bye");
    /// ```
    pub fn with_data<T: Into<Cow<'static, str>>>(mut self, data: T) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Sets or replaces the value of the `comment` field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    ///
    /// // The comment "🚀" will be sent.
    /// let event = Event::comment("Rocket is great!").with_comment("🚀");
    ///
    /// // The two below are equivalent.
    /// let event = Event::comment("bye").with_data("goodbye");
    /// let event = Event::data("goodbyte").with_comment("bye");
    /// ```
    pub fn with_comment<T: Into<Cow<'static, str>>>(mut self, data: T) -> Self {
        self.comment = Some(data.into());
        self
    }

    /// Sets or replaces the value of the `retry` field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::Event;
    /// use rocket::tokio::time::Duration;
    ///
    /// // The reconnection will be set to 10 seconds.
    /// let event = Event::retry(Duration::from_millis(500))
    ///     .with_retry(Duration::from_secs(10));
    ///
    /// // The two below are equivalent.
    /// let event = Event::comment("bye").with_retry(Duration::from_millis(500));
    /// let event = Event::retry(Duration::from_millis(500)).with_comment("bye");
    /// ```
    pub fn with_retry(mut self, period: Duration) -> Self {
        self.retry = Some(period);
        self
    }

    fn into_stream(self) -> impl Stream<Item = RawLinedEvent> {
        let events = [
            self.comment.map(|v| RawLinedEvent::many("", v)),
            self.retry.map(|r| RawLinedEvent::one("retry", format!("{}", r.as_millis()))),
            self.id.map(|v| RawLinedEvent::one("id", v)),
            self.event.map(|v| RawLinedEvent::one("event", v)),
            self.data.map(|v| RawLinedEvent::many("data", v)),
            Some(RawLinedEvent::raw("")),
        ];

        stream::iter(events).filter_map(ready)
    }
}

/// A potentially infinite stream of Server-Sent [`Event`]s (SSE).
///
/// An `EventStream` can be constructed from any [`Stream`] of items of type
/// `Event`. The stream can be constructed directly via [`EventStream::from()`]
/// or through generator syntax via [`EventStream!`].
///
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
///
/// # Responder
///
/// `EventStream` is a (potentially infinite) responder. The response
/// `Content-Type` is set to [`EventStream`](ContentType::EventStream). The body
/// is [unsized](crate::response::Body#unsized), and values are sent as soon as
/// they are yielded by the internal iterator.
///
/// ## Heartbeat
///
/// A heartbeat comment is injected into the internal stream and sent at a fixed
/// interval. The comment is discarded by clients and serves only to keep the
/// connection alive; it does not interfere with application data. The interval
/// defaults to 30 seconds but can be adjusted with
/// [`EventStream::heartbeat()`].
///
/// # Examples
///
/// Use [`EventStream!`] to yield an infinite series of "ping" SSE messages to
/// the client, one per second:
///
/// ```rust
/// # use rocket::*;
/// use rocket::response::stream::{Event, EventStream};;
/// use rocket::tokio::time::{self, Duration};
///
/// #[get("/events")]
/// fn stream() -> EventStream![] {
///     EventStream! {
///         let mut interval = time::interval(Duration::from_secs(1));
///         loop {
///             yield Event::data("ping");
///             interval.tick().await;
///         }
///     }
/// }
/// ```
///
/// Yield 9 events: 3 triplets of `retry`, `data`, and `comment` events:
///
/// ```rust
/// # use rocket::get;
/// use rocket::response::stream::{Event, EventStream};
/// use rocket::tokio::time::Duration;
///
/// #[get("/events")]
/// fn events() -> EventStream![] {
///     EventStream! {
///         for i in 0..3 {
///             yield Event::retry(Duration::from_secs(10));
///             yield Event::data(format!("{}", i)).id("cat").event("bar");
///             yield Event::comment("silly boy");
///         }
///     }
/// }
/// ```
///
/// The syntax of `EventStream!` as an expression is identical to that of
/// [`stream!`](crate::response::stream::stream). For how to gracefully
/// terminate an otherwise infinite stream, see [graceful
/// shutdown](crate::response::stream#graceful-shutdown).
///
/// # Borrowing
///
/// If an `EventStream` contains a borrow, the extended type syntax
/// `EventStream![Event + '_]` must be used:
///
/// ```rust
/// # use rocket::get;
/// use rocket::State;
/// use rocket::response::stream::{Event, EventStream};
///
/// #[get("/events")]
/// fn events(ctxt: &State<bool>) -> EventStream![Event + '_] {
///     EventStream! {
///         // By using `ctxt` in the stream, the borrow is moved into it. Thus,
///         // the stream object contains a borrow, prompting the '_ annotation.
///         if *ctxt.inner() {
///             yield Event::data("hi");
///         }
///     }
/// }
/// ```
///
/// See [`stream#borrowing`](crate::response::stream#borrowing) for further
/// details on borrowing in streams.
///
/// # Pitfalls
///
/// Server-Sent Events are a rather simple mechanism, though there are some
/// pitfalls to be aware of.
///
///  * **Buffering**
///
///    Protocol restrictions complicate implementing an API that does not
///    buffer. As such, if you are sending _lots_ of data, consider sending the
///    data via multiple data fields (with events to signal start and end).
///    Alternatively, send _one_ event which instructs the client to fetch the
///    data from another endpoint which in-turn streams the data.
///
///  * **Raw SSE requires UTF-8 data**
///
///    Only UTF-8 data can be sent via SSE. If you need to send arbitrary bytes,
///    consider encoding it, for instance, as JSON using [`Event::json()`].
///    Alternatively, as described before, use SSE as a notifier which alerts
///    the client to fetch the data from elsewhere.
///
///  * **Raw SSE is Lossy**
///
///    Data sent via SSE cannot contain new lines `\n` or carriage returns `\r`
///    due to interference with the line protocol.
///
///    The protocol allows expressing new lines as multiple messages, however,
///    and Rocket automatically transforms a message of `foo\nbar` into two
///    messages, `foo` and `bar`, so that they are reconstructed (automatically)
///    as `foo\nbar` on the client-side. For messages that only contain new
///    lines `\n`, the conversion is lossless.
///
///    However, the protocol has no mechanism for expressing carriage returns
///    and thus it is not possible to send unencoded carriage returns via SSE.
///    Rocket handles carriage returns like it handles new lines: it splits the
///    data into multiple messages. Thus, a sequence of `\r\n` becomes `\n` at
///    the client side. A single `\r` that is not part of an `\r\n` sequence
///    also becomes `\n` at the client side. As a result, the message
///    `foo\r\nbar\rbaz` is read as `foo\nbar\nbaz` at the client-side.
///
///    To send messages losslessly, they must be encoded first, for instance, by
///    using [`Event::json()`].
pub struct EventStream<S> {
    stream: S,
    heartbeat: Option<Duration>,
}

impl<S: Stream<Item = Event>> EventStream<S> {
    /// Sets a "ping" interval for this `EventStream` to avoid connection
    /// timeouts when no data is being transferred. The default `interval` is 30
    /// seconds.
    ///
    /// The ping is implemented by sending an empty comment to the client every
    /// `interval` seconds.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// use rocket::response::stream::{Event, EventStream};
    /// use rocket::tokio::time::Duration;
    ///
    /// #[get("/events")]
    /// fn events() -> EventStream![] {
    ///     // Remove the default heartbeat.
    ///     # let event_stream = rocket::futures::stream::pending();
    ///     EventStream::from(event_stream).heartbeat(None);
    ///
    ///     // Set the heartbeat interval to 15 seconds.
    ///     # let event_stream = rocket::futures::stream::pending();
    ///     EventStream::from(event_stream).heartbeat(Duration::from_secs(15));
    ///
    ///     // Do the same but for a generated `EventStream`:
    ///     let stream = EventStream! {
    ///         yield Event::data("hello");
    ///     };
    ///
    ///     stream.heartbeat(Duration::from_secs(15))
    /// }
    /// ```
    pub fn heartbeat<H: Into<Option<Duration>>>(mut self, heartbeat: H) -> Self {
        self.heartbeat = heartbeat.into();
        self
    }

    fn heartbeat_stream(&self) -> Option<impl Stream<Item = RawLinedEvent>> {
        use tokio::time::interval;
        use tokio_stream::wrappers::IntervalStream;

        self.heartbeat
            .map(|beat| IntervalStream::new(interval(beat)))
            .map(|stream| stream.map(|_| RawLinedEvent::raw(":")))
    }

    fn into_stream(self) -> impl Stream<Item = RawLinedEvent> {
        use futures::future::Either;
        use crate::ext::StreamExt;

        let heartbeat_stream = self.heartbeat_stream();
        let raw_events = self.stream.map(|e| e.into_stream()).flatten();
        match heartbeat_stream {
            Some(heartbeat) => Either::Left(raw_events.join(heartbeat)),
            None => Either::Right(raw_events)
        }
    }

    fn into_reader(self) -> impl AsyncRead {
        ReaderStream::from(self.into_stream())
    }
}

impl<S: Stream<Item = Event>> From<S> for EventStream<S> {
    /// Creates an `EventStream` from a [`Stream`] of [`Event`]s.
    ///
    /// Use `EventStream::from()` to construct an `EventStream` from an already
    /// existing stream. Otherwise, prefer to use [`EventStream!`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::stream::{Event, EventStream};
    /// use rocket::futures::stream;
    ///
    /// let raw = stream::iter(vec![Event::data("a"), Event::data("b")]);
    /// let stream = EventStream::from(raw);
    /// ```
    fn from(stream: S) -> Self {
        EventStream { stream, heartbeat: Some(Duration::from_secs(30)), }
    }
}

impl<'r, S: Stream<Item = Event> + Send + 'r> Responder<'r, 'r> for EventStream<S> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::EventStream)
            .raw_header("Cache-Control", "no-cache")
            .raw_header("Expires", "0")
            .streamed_body(self.into_reader())
            .ok()
    }
}

crate::export! {
    /// Type and stream expression macro for [`struct@EventStream`].
    ///
    /// See [`stream!`](crate::response::stream::stream) for the syntax
    /// supported by this macro. In addition to that syntax, this macro can also
    /// be called with no arguments, `EventStream![]`, as shorthand for
    /// `EventStream![Event]`.
    ///
    /// See [`struct@EventStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! EventStream {
        () => ($crate::_typed_stream!(EventStream, $crate::response::stream::Event));
        ($($s:tt)*) => ($crate::_typed_stream!(EventStream, $($s)*));
    }
}

#[cfg(test)]
mod sse_tests {
    use tokio::io::AsyncReadExt;
    use tokio::time::{self, Duration};
    use futures::stream::Stream;
    use crate::response::stream::{stream, Event, EventStream, ReaderStream};

    impl Event {
        fn into_string(self) -> String {
            crate::async_test(async move {
                let mut string = String::new();
                let mut reader = ReaderStream::from(self.into_stream());
                reader.read_to_string(&mut string).await.expect("event -> string");
                string
            })
        }
    }

    impl<S: Stream<Item = Event>> EventStream<S> {
        fn into_string(self) -> String {
            crate::async_test(async move {
                let mut string = String::new();
                let reader = self.into_reader();
                tokio::pin!(reader);
                reader.read_to_string(&mut string).await.expect("event stream -> string");
                string
            })
        }
    }

    #[test]
    fn test_event_data() {
        let event = Event::data("a\nb");
        assert_eq!(event.into_string(), "data:a\ndata:b\n\n");

        let event = Event::data("a\n");
        assert_eq!(event.into_string(), "data:a\ndata:\n\n");

        let event = Event::data("cats make me happy!");
        assert_eq!(event.into_string(), "data:cats make me happy!\n\n");

        let event = Event::data("in the\njungle\nthe mighty\njungle");
        assert_eq!(event.into_string(),
            "data:in the\ndata:jungle\ndata:the mighty\ndata:jungle\n\n");

        let event = Event::data("in the\njungle\r\nthe mighty\rjungle");
        assert_eq!(event.into_string(),
            "data:in the\ndata:jungle\ndata:the mighty\ndata:jungle\n\n");

        let event = Event::data("\nb\n");
        assert_eq!(event.into_string(), "data:\ndata:b\ndata:\n\n");

        let event = Event::data("\r\nb\n");
        assert_eq!(event.into_string(), "data:\ndata:b\ndata:\n\n");

        let event = Event::data("\r\nb\r\n");
        assert_eq!(event.into_string(), "data:\ndata:b\ndata:\n\n");

        let event = Event::data("\n\nb\n");
        assert_eq!(event.into_string(), "data:\ndata:\ndata:b\ndata:\n\n");

        let event = Event::data("\n\rb\n");
        assert_eq!(event.into_string(), "data:\ndata:\ndata:b\ndata:\n\n");

        let event = Event::data("\n\rb\r");
        assert_eq!(event.into_string(), "data:\ndata:\ndata:b\ndata:\n\n");

        let event = Event::comment("\n\rb\r");
        assert_eq!(event.into_string(), ":\n:\n:b\n:\n\n");

        let event = Event::data("\n\n\n");
        assert_eq!(event.into_string(), "data:\ndata:\ndata:\ndata:\n\n");

        let event = Event::data("\n");
        assert_eq!(event.into_string(), "data:\ndata:\n\n");

        let event = Event::data("");
        assert_eq!(event.into_string(), "data:\n\n");
    }

    #[test]
    fn test_event_fields() {
        let event = Event::data("foo").id("moo");
        assert_eq!(event.into_string(), "id:moo\ndata:foo\n\n");

        let event = Event::data("foo").id("moo").with_retry(Duration::from_secs(45));
        assert_eq!(event.into_string(), "retry:45000\nid:moo\ndata:foo\n\n");

        let event = Event::data("foo\nbar").id("moo").with_retry(Duration::from_secs(45));
        assert_eq!(event.into_string(), "retry:45000\nid:moo\ndata:foo\ndata:bar\n\n");

        let event = Event::retry(Duration::from_secs(45));
        assert_eq!(event.into_string(), "retry:45000\n\n");

        let event = Event::comment("incoming data...");
        assert_eq!(event.into_string(), ":incoming data...\n\n");

        let event = Event::data("foo").id("moo").with_comment("cows, ey?");
        assert_eq!(event.into_string(), ":cows, ey?\nid:moo\ndata:foo\n\n");

        let event = Event::data("foo\nbar")
            .id("moo")
            .event("milk")
            .with_retry(Duration::from_secs(3));

        assert_eq!(event.into_string(), "retry:3000\nid:moo\nevent:milk\ndata:foo\ndata:bar\n\n");

        let event = Event::data("foo")
            .id("moo")
            .event("milk")
            .with_comment("??")
            .with_retry(Duration::from_secs(3));

        assert_eq!(event.into_string(), ":??\nretry:3000\nid:moo\nevent:milk\ndata:foo\n\n");

        let event = Event::data("foo")
            .id("moo")
            .event("milk")
            .with_comment("?\n?")
            .with_retry(Duration::from_secs(3));

        assert_eq!(event.into_string(), ":?\n:?\nretry:3000\nid:moo\nevent:milk\ndata:foo\n\n");

        let event = Event::data("foo\r\nbar\nbaz")
            .id("moo")
            .event("milk")
            .with_comment("?\n?")
            .with_retry(Duration::from_secs(3));

        assert_eq!(event.into_string(),
            ":?\n:?\nretry:3000\nid:moo\nevent:milk\ndata:foo\ndata:bar\ndata:baz\n\n");
    }

    #[test]
    fn test_bad_chars() {
        let event = Event::data("foo").id("dead\nbeef").event("m\noo");
        assert_eq!(event.into_string(), "id:dead beef\nevent:m oo\ndata:foo\n\n");

        let event = Event::data("f\no").id("d\r\nbe\rf").event("m\n\r");
        assert_eq!(event.into_string(), "id:d  be f\nevent:m  \ndata:f\ndata:o\n\n");

        let event = Event::data("f\no").id("\r\n\n\r\n\r\r").event("\n\rb");
        assert_eq!(event.into_string(), "id:       \nevent:  b\ndata:f\ndata:o\n\n");
    }

    #[test]
    fn test_event_stream() {
        use futures::stream::iter;

        let stream = EventStream::from(iter(vec![Event::data("foo")]));
        assert_eq!(stream.into_string().replace(":\n\n", ""), "data:foo\n\n");

        let stream = EventStream::from(iter(vec![Event::data("a"), Event::data("b")]));
        assert_eq!(stream.into_string().replace(":\n\n", ""), "data:a\n\ndata:b\n\n");

        let stream = EventStream::from(iter(vec![
                Event::data("a\nb"),
                Event::data("b"),
                Event::data("c\n\nd"),
                Event::data("e"),
        ]));

        assert_eq!(stream.into_string().replace(":\n\n", ""),
            "data:a\ndata:b\n\ndata:b\n\ndata:c\ndata:\ndata:d\n\ndata:e\n\n");
    }

    #[test]
    fn test_heartbeat() {
        use futures::future::ready;
        use futures::stream::{once, iter, StreamExt};

        const HEARTBEAT: &str = ":\n";

        // Set a heartbeat interval of 250ms. Send nothing for 600ms. We should
        // get 2 or 3 heartbeats, the latter if one is sent eagerly. Maybe 4.
        let raw = stream!(time::sleep(Duration::from_millis(600)).await;)
            .map(|_| unreachable!());

        let string = EventStream::from(raw)
            .heartbeat(Duration::from_millis(250))
            .into_string();

        let heartbeats = string.matches(HEARTBEAT).count();
        assert!(heartbeats >= 2 && heartbeats <= 4, "got {} beat(s)", heartbeats);

        let stream = EventStream! {
            time::sleep(Duration::from_millis(250)).await;
            yield Event::data("foo");
            time::sleep(Duration::from_millis(250)).await;
            yield Event::data("bar");
        };

        // We expect: foo\n\n [heartbeat] bar\n\n [maybe beartbeat].
        let string = stream.heartbeat(Duration::from_millis(350)).into_string();
        let heartbeats = string.matches(HEARTBEAT).count();
        assert!(heartbeats >= 1 && heartbeats <= 3, "got {} beat(s)", heartbeats);
        assert!(string.contains("data:foo\n\n"), "string = {:?}", string);
        assert!(string.contains("data:bar\n\n"), "string = {:?}", string);

        // We shouldn't send a heartbeat if a message is immediately available.
        let stream = EventStream::from(once(ready(Event::data("hello"))));
        let string = stream.heartbeat(Duration::from_secs(1)).into_string();
        assert_eq!(string, "data:hello\n\n", "string = {:?}", string);

        // It's okay if we do it with two, though.
        let stream = EventStream::from(iter(vec![Event::data("a"), Event::data("b")]));
        let string = stream.heartbeat(Duration::from_secs(1)).into_string();
        let heartbeats = string.matches(HEARTBEAT).count();
        assert!(heartbeats <= 1);
        assert!(string.contains("data:a\n\n"), "string = {:?}", string);
        assert!(string.contains("data:b\n\n"), "string = {:?}", string);
    }
}
