# Rocket Website Source

This directory contains the source files for the content on [Rocket's
website](https://rocket.rs).

## Contents

This directory contains the following:

  * `index.toml` - Source data for the index.
  * `overview.toml` - Source data for the overview page (`overview/`).
  * `news/index.toml` - Source data for the news page (`news/`).
  * `news/*.md` - News articles linked to from `news/index.toml`.
  * `guide/*.md` - Guide pages linked to from `guide.md`.

[Rocket Programming Guide]: https://rocket.rs/v0.5-rc/guide/

### Guide Links

Cross-linking guide pages is accomplished via relative links. Outside of the
index, this is: `../{page}#anchor`. For instance, to link to the **Quickstart >
Running Examples** page, use `../quickstart#running-examples`.

### Aliases

Aliases are shorthand URLs that start with `@` (e.g, `@api`). They are used
throughout the guide to simplify versioning URLs to Rocket's source code and the
Rocket API. They are replaced at build time with a URL prefix. At present, the
following aliases are available, where `${version}` is Rocket's version string
at the time of compilation:

  * `@example`: https://github.com/SergioBenitez/Rocket/tree/${version}/examples
  * `@github`: https://github.com/SergioBenitez/Rocket/tree/${version}
  * `@api`: https://api.rocket.rs/${version}

For example, to link to `Rocket::launch()`, you might write:

```md
Launch an instance of your application using the [`launch()`] method.

[`launch()`]: @api/rocket/struct.Rocket.html#method.launch
```

## License

The Rocket website source is licensed under the [GNU General Public License v3.0](LICENSE).
