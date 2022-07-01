use std::borrow::Cow;
use std::io::{self, Cursor};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::cmp::min;

use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf, Take};

/// Low-level serialization of fields in text/event-stream format.
///
/// Based on https://html.spec.whatwg.org/multipage/server-sent-events.html,
/// reproduced here for quick reference. Retrieved 2021-04-17.
///
/// ```text
/// stream        = [ bom ] *event
/// event         = *( comment / field ) end-of-line
/// comment       = colon *any-char end-of-line
/// field         = 1*name-char [ colon [ space ] *any-char ] end-of-line
/// end-of-line   = ( cr lf / cr / lf )
///
/// ; characters
/// lf            = %x000A ; U+000A LINE FEED (LF)
/// cr            = %x000D ; U+000D CARRIAGE RETURN (CR)
/// space         = %x0020 ; U+0020 SPACE
/// colon         = %x003A ; U+003A COLON (:)
/// bom           = %xFEFF ; U+FEFF BYTE ORDER MARK
/// name-char     = %x0000-0009 / %x000B-000C / %x000E-0039 / %x003B-10FFFF
///                 ; a scalar value other than:
///                 ; U+000A LINE FEED (LF), U+000D CARRIAGE RETURN (CR), or U+003A COLON (:)
/// any-char      = %x0000-0009 / %x000B-000C / %x000E-10FFFF
///                 ; a scalar value other than:
///                 ; U+000A LINE FEED (LF) or U+000D CARRIAGE RETURN (CR)/
/// ```
///
/// Notice that Multiple encodings are possible for the same data, especially in
/// the choice of newline. This implementation always uses only "\n" (LF).
///
/// Serializes (via `AsyncRead`) as a series of "${name}:${value}\n" events.
/// Either or both `name` and `value` may be empty. When the name is empty, this
/// is a comment. Otherwise, this is a field.
#[derive(Debug)]
pub struct RawLinedEvent {
    name: Cursor<Cow<'static, [u8]>>,
    value: Take<Cursor<Cow<'static, [u8]>>>,
    state: State,
}

/// Converts a `Cow<str>` to a `Cow<[u8]>`.
fn farm(cow: Cow<'_, str>) -> Cow<'_, [u8]> {
    match cow {
        Cow::Borrowed(slice) => Cow::Borrowed(slice.as_bytes()),
        Cow::Owned(vec) => Cow::Owned(vec.into_bytes())
    }
}

/// Farms `cow`, replacing `\r`, `\n`, and `:` with ` ` in the process.
///
/// This converts any string into a valid event `name`.
fn farm_name(cow: Cow<'_, str>) -> Cow<'_, [u8]> {
    let mut i = 0;
    let mut cow = farm(cow);
    while i < cow.len() {
        if let Some(k) = memchr::memchr3(b'\r', b'\n', b':', &cow[i..]) {
            cow.to_mut()[i + k] = b' ';
            // This can't overflow as i + k + 1 <= len, since we found a char.
            i += k + 1;
        } else {
            break;
        }
    }

    cow
}

/// Farms `cow`, replacing `\r` and `\n` with ` ` in the process.
///
/// This converts any string into a valid event `value`.
fn farm_value(cow: Cow<'_, str>) -> Cow<'_, [u8]> {
    let mut i = 0;
    let mut cow = farm(cow);
    while i < cow.len() {
        if let Some(k) = memchr::memchr2(b'\r', b'\n', &cow[i..]) {
            cow.to_mut()[i + k] = b' ';
            // This can't overflow as i + k + 1 <= len, since we found a char.
            i += k + 1;
        } else {
            break;
        }
    }

    cow
}

impl RawLinedEvent {
    /// Create a `RawLinedEvent` from a valid, prefarmed `name` and `value`.
    fn prefarmed(name: Cow<'static, [u8]>, value: Cow<'static, [u8]>) -> RawLinedEvent {
        let name = Cursor::new(name);
        let mut value = Cursor::new(value).take(0);
        advance(&mut value);
        RawLinedEvent { name, value, state: State::Name }
    }

    /// Create a `RawLinedEvent` from potentially invalid `name` and `value`
    /// where `value` is not allowed to be multiple lines.
    ///
    /// Characters `\n`, `\r`, and ':' in `name` and characters `\r` \`n` in
    /// `value` `are replaced with a space ` `.
    pub fn one<N, V>(name: N, value: V) -> RawLinedEvent
        where N: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>
    {
        RawLinedEvent::prefarmed(farm_name(name.into()), farm_value(value.into()))
    }

    /// Create a `RawLinedEvent` from potentially invalid `name` and `value`
    /// where `value` is allowed to be multiple lines.
    ///
    /// Characters `\n`, `\r`, and ':' in `name` are replaced with a space ` `.
    /// `value` is allowed to contain any character. New lines (`\r\n` or `\n`)
    /// and carriage returns `\r` result in a new event being emitted.
    pub fn many<N, V>(name: N, value: V) -> RawLinedEvent
        where N: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>
    {
        RawLinedEvent::prefarmed(farm_name(name.into()), farm(value.into()))
    }

    /// Create a `RawLinedEvent` from known value `value`. The value is emitted
    /// directly with _no_ name and suffixed with a `\n`.
    pub fn raw<V: Into<Cow<'static, str>>>(value: V) -> RawLinedEvent {
        let value = value.into();
        let len = value.len();
        RawLinedEvent {
            name: Cursor::new(Cow::Borrowed(&[])),
            value: Cursor::new(farm(value)).take(len as u64),
            state: State::Value
        }
    }
}

/// The `AsyncRead`er state.
#[derive(Debug, PartialEq, Copy, Clone)]
enum State {
    Name,
    Colon,
    Value,
    NewLine,
    Done
}

/// Find the next new-line (`\n` or `\r`) character in `buf` beginning at the
/// current cursor position and sets the limit to be at that position.
fn advance<T: AsRef<[u8]> + Unpin>(buf: &mut Take<Cursor<T>>) {
    // Technically, the position need not be <= len, so we right it.
    let pos = min(buf.get_ref().get_ref().as_ref().len() as u64, buf.get_ref().position());
    let inner = buf.get_ref().get_ref().as_ref();
    let next = memchr::memchr2(b'\n', b'\r', &inner[(pos as usize)..])
        .map(|i| pos + i as u64)
        .unwrap_or_else(|| inner.len() as u64);

    let limit = next - pos;
    buf.set_limit(limit);
}

/// If the cursor in `buf` is currently at an `\r`, `\r\n` or `\n`, sets the
/// cursor position to be _after_ the characters.
fn skip<T: AsRef<[u8]> + Unpin>(buf: &mut Take<Cursor<T>>) {
    let pos = min(buf.get_ref().get_ref().as_ref().len() as u64, buf.get_ref().position());
    match buf.get_ref().get_ref().as_ref().get(pos as usize) {
        // This cannot overflow as clearly `buf.len() >= pos + 1`.
        Some(b'\n') => buf.get_mut().set_position(pos + 1),
        Some(b'\r') => {
            let next = (pos as usize).saturating_add(1);
            if buf.get_ref().get_ref().as_ref().get(next) == Some(&b'\n') {
                // This cannot overflow as clearly `buf.len() >= pos + 2`.
                buf.get_mut().set_position(pos + 2);
            } else {
                // This cannot overflow as clearly `buf.len() >= pos + 1`.
                buf.get_mut().set_position(pos + 1);
            }
        }
        _ => return,
    }
}


macro_rules! dbg_assert_ready {
    ($e:expr) => ({
        let poll = $e;
        debug_assert!(poll.is_ready());
        ::futures::ready!(poll)
    })
}

// NOTE: The correctness of this implementation depends on the types of `name`
// and `value` having `AsyncRead` implementations that always return `Ready`.
// Otherwise, we may return `Pending` after having written data to `buf` which
// violates the contract. This can happen because even after a successful
// partial or full read of `name`, we loop back to a `ready!(name.poll())` if
// `buf` was not completely filled. So, we return `Pending` if that poll does.
impl AsyncRead for RawLinedEvent {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        use bytes::Buf;

        loop {
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }

            match self.state {
                State::Name => {
                    dbg_assert_ready!(Pin::new(&mut self.name).poll_read(cx, buf))?;
                    if !self.name.has_remaining() {
                        self.name.set_position(0);
                        self.state = State::Colon;
                    }
                }
                State::Colon => {
                    // Note that we've checked `buf.remaining() != 0`.
                    buf.put_slice(&[b':']);
                    self.state = State::Value;
                }
                State::Value => {
                    dbg_assert_ready!(Pin::new(&mut self.value).poll_read(cx, buf))?;
                    if self.value.limit() == 0 {
                        self.state = State::NewLine;
                    }
                }
                State::NewLine => {
                    // Note that we've checked `buf.remaining() != 0`.
                    buf.put_slice(&[b'\n']);
                    if self.value.get_ref().has_remaining() {
                        skip(&mut self.value);
                        advance(&mut self.value);
                        self.state = State::Name;
                    } else {
                        self.state = State::Done;
                    }
                }
                State::Done => return Poll::Ready(Ok(()))
            }
        }
    }
}
