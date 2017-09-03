# Rocket Website Source

This directory contains the source files for the content on [Rocket's
website](https://rocket.rs).

## Contents

This directory contains the following:

  * `index.toml` - Source data for the index (`/`).
  * `news.toml` - Source data for the news page (`/news`).
  * `overview.toml` - Source data for the overview page (`/overview`).
  * `guide.md` - Index page for the [Rocket Programming Guide] (`/guide`).
  * `news/*.md` - News articles linked to from `news.toml`.
  * `guide/*.md` - Guide pages linked to from `guide.md`.

[Rocket Programming Guide]: https://rocket.rs/guide/

### Guide Links

Cross-linking to pages in the guide is accomplished via absolute links rooted at
`/guide/`. To link to the page whose source is at `guide/page.md`, for instance,
link to `/guide/page`.

## License

The Rocket website source is licensed under the [GNU General Public License v3.0](LICENSE).
