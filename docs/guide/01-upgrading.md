+++
summary = "a migration guide from Rocket v0.5 to v0.6"
+++

# Upgrading

This a placeholder for an eventual migration guide from v0.5 to v0.6.

## Typed Catchers

The largest change from 0.5 to 0.6 is a complete overhaul of the error catching
mechanism. While most catchers should continue to function as expected, the
new functionality completely changes how errors should be handled.

When forwarding or generating an error, every guard is required to provide a typed
error. Previously, errors were forwarded to catchers based on the URI, format, and
status code. Now, since every error has an associated typed error, catchers can
specialize on the error type, and will even recieve access to the typed error
when generating a response.

<!-- TODO: Compelte notes with examples -->

## Getting Help

If you run into any issues upgrading, we encourage you to ask questions via
[GitHub discussions] or via chat at [`#rocket:mozilla.org`] on Matrix. The
[FAQ](../faq/) also provides answers to commonly asked questions.

[GitHub discussions]: @github/discussions
[`#rocket:mozilla.org`]: @chat
