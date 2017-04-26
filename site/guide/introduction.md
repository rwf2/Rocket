# Introduction

Rocket is a web framework for Rust. If you'd like, you can think of Rocket as
being a more flexible, friendly medley of [Rails](http://rubyonrails.org),
[Flask](http://flask.pocoo.org/),
[Bottle](http://bottlepy.org/docs/dev/index.html), and
[Yesod](http://www.yesodweb.com/). We prefer to think of Rocket as something
new. Rocket aims to be fast, easy, and flexible. It also aims to be _fun_, and
it accomplishes this by ensuring that you write as little code as needed to
accomplish your task. This guide introduces you to the core, intermediate, and
advanced concepts of Rocket. After reading this guide, you should find yourself
being _very_ productive with Rocket.

## Audience

Readers are assumed to have a good grasp of the Rust programming language.
Readers new to Rust are encouraged to read the [Rust
Book](https://doc.rust-lang.org/book/). This guide also assumes a basic
understanding of web application fundamentals, such as routing and HTTP.

## Foreword

Rocket's design is centered around three core philosophies:

  * **Function declaration and parameter types should contain all the necessary
    information to validate and process a request.** This immediately prohibits
    APIs where request state is retrieved from a global context. As a result,
    request handling is **self-contained** in Rocket: handlers are regular
    functions with regular arguments.

  * **All request handling information should be typed.** Because the web and
    HTTP are themselves untyped (or _stringly_ typed, as some call it), this
    means that something or someone has to convert strings to native types.
    Rocket does this for you with zero programming overhead.

  * **Decisions should not be forced.** Templates, serialization, sessions, and
    just about everything else are all pluggable, optional components. While
    Rocket has official support and libraries for each of these, they are
    completely optional and swappable.

These three ideas dictate Rocket's interface, and you will find the ideas
embedded in Rocket's core features.

