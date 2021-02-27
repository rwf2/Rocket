# Introduction

Rocket is a web framework for Rust. If you'd like, you can think of Rocket as
being a more flexible, friendly medley of [Rails](https://rubyonrails.org/),
[Flask](https://palletsprojects.com/p/flask/),
[Bottle](https://bottlepy.org/docs/dev/index.html), and
[Yesod](https://www.yesodweb.com/). We prefer to think of Rocket as something
new. Rocket aims to be fast, easy, and flexible while offering guaranteed safety
and security where it can. Importantly, Rocket also aims to be _fun_, and it
accomplishes this by ensuring that you write as little code as needed to
accomplish your task.

This guide introduces you to the core, intermediate, and advanced concepts of
Rocket. After reading this guide, you should find yourself being very
productive with Rocket.

## Audience

Readers are assumed to have a good grasp of the Rust programming language.
Readers new to Rust are encouraged to read the [Rust
Book](https://doc.rust-lang.org/book/). This guide also assumes a basic
understanding of web application fundamentals, such as routing and HTTP. Mozilla
provides a good overview of these concepts in their [MDN web docs].

[MDN web docs]: https://developer.mozilla.org/en-US/docs/Web/HTTP

## Foreword

Rocket's design is centered around three core philosophies:

  * **Security, correctness, and developer experience are paramount.**

    The path of least resistance should lead you to the most secure, correct web
    application, though security and correctness should not come at the cost of
    a degraded developer experience. Rocket is easy to use while taking great
    measures to ensure that your application is secure and correct without
    cognitive overhead.

  * **All request handling information should be typed and self-contained.**

    Because the web and HTTP are themselves untyped (or _stringly_ typed, as
    some call it), this means that something or someone has to convert strings
    to native types. Rocket does this for you with zero programming overhead.
    What's more, Rocket's request handling is **self-contained** with zero
    global state: handlers are regular functions with regular arguments.

  * **Decisions should not be forced.**

    Templates, serialization, sessions, and just about everything else are all
    pluggable, optional components. While Rocket has official support and
    libraries for each of these, they are completely optional and swappable.

These three ideas dictate Rocket's interface, and you will find all of them
embedded in Rocket's core features.
