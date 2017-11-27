# Rocket MVC

A REST web application build in [Rust](https://www.rust-lang.org/). It use [Rocket](https://rocket.rs) & [Diesel](http://diesel.rs/).

This sample just an applicatio who allow you to edit recipes according to [REST](https://en.wikipedia.org/wiki/Representational_state_transfer) conventions.

- **List** recipes: GET /recipes 
- **Create** recipes: POST /recipes
- **Show** recipe: GET /recipes/<recipe_id>
- **New** recipe: GET /recipes/new
- **Edit** recipe: GET /recipes/<recipe_id>/edit
- **Update** recipe: PUT /recipes/<recipe_id>
- **Delete** recipe: DELETE /recipes/<recipe_id>

## Instalation

First you need to [install Rust](https://www.rust-lang.org/install.html). Then Clone the repository and go in the folder

~~~bash
$ git clone https://github.com/madeindjs/Rocket_MVC.git
$ cd Rocket_MVC
~~~

According to [Rocket](https://rocket.rs), you need to use Nightly version of [Rust](https://www.rust-lang.org/)

~~~bash
$ rustup override set nightly
~~~

Then install [diesel_cli](https://github.com/diesel-rs/diesel/tree/master/diesel_cli) and run database migrations

~~~bash
$ cargo install diesel_cli --no-default-features --features sqlite
$ diesel setup
~~~

And now you can build project

~~~bash
$ cargo run
~~~


## Benchmark

## License

[MIT](https://opensource.org/licenses/MIT)

[Rust]: https://www.rust-lang.org/
