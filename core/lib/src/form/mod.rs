//! Parsing and validation of HTTP forms and fields.
//!
//! See the [forms guide](https://rocket.rs/v0.5-rc/guide/requests#forms) for
//! general form support documentation.
//!
//! # Field Wire Format
//!
//! Rocket's field wire format is a flexible, non-self-descriptive, text-based
//! encoding of arbitrarily nested structure keys and their corresponding
//! values. The general grammar is:
//!
//! ```ebnf
//! field := name ('=' value)?
//!
//! name := key*
//!
//! key := indices
//!       | '[' indices ']'
//!       | '.' indices
//!
//! indices := index (':' index)*
//!
//! index := STRING except ':', ']'
//!
//! value := STRING
//! ```
//!
//! Each field name consists of any number of `key`s and at most one `value`.
//! Keys are delimited by `[` or `.`. A `key` consists of indices delimited by
//! `:`.
//!
//! The meaning of a key or index is type-dependent, hence the format is
//! non-self-descriptive. _Any_ structure can be described by this format. The
//! delimiters `.`, `[`, `:`, and `]` have no semantic meaning.
//!
//! Some examples of valid fields are:
//!
//!   * `=`
//!   * `key=value`
//!   * `key[]=value`
//!   * `.0=value`
//!   * `[0]=value`
//!   * `people[].name=Bob`
//!   * `bob.cousin.names[]=Bob`
//!   * `map[k:1]=Bob`
//!   * `people[bob]nickname=Stan`
//!
//! See [`FromForm`] for full details on push-parsing and complete examples.

// ## Maps w/named Fields (`struct`)
//
// A `struct` with named fields parses values of multiple types, indexed by the
// name of its fields:
//
// ```rust,ignore
// struct Dog { name: String, barks: bool, friends: Vec<Cat>, }
// struct Cat { name: String, meows: bool }
// ```
//
// Candidates for parsing into a `Dog` include:
//
//   * `name=Fido&barks=0`
//
//     `Dog { "Fido", false }`
//
//   * `name=Fido&barks=1&friends[0]name=Sally&friends[0]meows=0`
//     `name=Fido&barks=1&friends[0].name=Sally&friends[0].meows=0`
//     `name=Fido&barks=1&friends.0.name=Sally&friends.0.meows=0`
//
//     `Dog { "Fido", true, vec![Cat { "Sally", false }] }`
//
// Parsers for structs are code-generated to proceed as follows:
//
//   1. **Initialization.** The context stores parsing options, a `T::Context`
//      for each field of type `T`, and a vector called `extra`.
//
//      ```rust,ignore
//      struct Context<'v> {
//          opts: FormOptions,
//          field_a: A::Context,
//          field_b: B::Context,
//          /* ... */
//          extra: Vec<FormField<'v>>
//      }
//      ```
//
//   2. **Push.** The index of the first key is compared to known field names.
//      If none matches, the index is added to `extra`. Otherwise the key is
//      stripped from the field, and the remaining field is pushed to `T`.
//
//      ```rust,ignore
//      fn push(this: &mut Self::Context, field: FormField<'v>) {
//          match field.key() {
//              "field_a" => A::push(&mut this.field_a, field.next()),
//              "field_b" => B::push(&mut this.field_b, field.next()),
//              /* ... */
//              _ => this.extra.push(field)
//          }
//      }
//      ```
//
//   3. **Finalization.** Every context is finalized; errors and `Ok` values
//      are collected. If parsing is strict and extras is non-empty, an error
//      added to the collection of errors. If there are no errors, all `Ok`
//      values are used to create the `struct`, and the created struct is
//      returned. Otherwise, `Err(errors)` is returned.
//
//      ```rust,ignore
//      fn finalize(mut this: Self::Context) -> Result<Self, Self::Error> {
//          let mut errors = vec![];
//
//          let field_a = A::finalize(&mut this.field_a)
//             .map_err(|e| errors.push(e))
//             .map(Some).unwrap_or(None);
//
//          let field_b = B::finblize(&mut this.field_b)
//             .map_err(|e| errors.push(e))
//             .map(Some).unwrap_or(None);
//
//          /* .. */
//
//          if !errors.is_empty() {
//              return Err(Values(errors));
//          } else if this.opts.is_strict() && !this.extra.is_empty() {
//              return Err(Extra(this.extra));
//          } else {
//              // NOTE: All unwraps will succeed since `errors.is_empty()`.
//              Struct {
//                 field_a: field_a.unwrap(),
//                 field_b: field_b.unwrap(),
//                 /* .. */
//              }
//          }
//      }
//      ```
//
// ## Sequences: (`Vec<T: FromForm>`)
//
// A `Vec<T: FromForm>` invokes `T`'s push-parser on every push, adding instances
// of `T` to an internal vector. The instance of `T` whose parser is invoked
// depends on the index of the first key:
//
//   * if it is the first push, the index differs from the previous, or there is no
//     index, a new `T::Context` is `init`ialized and added to the internal vector
//   * if the index matches the previously seen index, the last initialized
//     `T::Context` is `push`ed to.
//
// For instance, the sequentially pushed values `=1`, `=2`, and `=3` for a
// `Vec<usize>` (or any other integer) is expected to parse as `vec![1, 2, 3]`. The
// same is true for `[]=1&[]=2&[]=3`. In the first example (`=1&..`), the fields
// passed to `Vec`'s push-parser (`=1`, ..) have no key and thus no index. In the
// second example (`[]=1&..`), the key is `[]` (`[]=1`) without an index. In both
// cases, there is no index. The `Vec` parser takes this to mean that a _new_ `T`
// should be parsed using the field's value.
//
// If, instead, the index was non-empty and equal to the index of the field in the
// _previous_ push, `Vec` pushes the value to the parser of the previously parsed
// `T`: `[]=1&[0]=2&[0]=3` results in `vec![1, 2]` and `[0]=1&[0]=2&[]=3` results
// in `vec![1, 3]` (see [`FromFormValue`]).
//
// This generalizes. Consider a `Vec<Vec<usize>>` named `x`, so `x` and an
// optional `=` are stripped before being passed to `Vec`'s push-parser:
//
//   * `x=1&x=2&x=3` parses as `vec![vec![1], vec![2], vec![3]]`
//
//     Every push (`1`, `2`, `3`) has no key, thus no index: a new `T` (here,
//     `Vec<usize>`) is thus initialized for every `push()` and passed the
//     value (here, `1`, `2`, and `3`). Each of these `push`es proceeds
//     recursively: every push again has no key, thus no index, so a new `T` is
//     initialized for every push (now a `usize`), which finally parse as
//     integers `1`, `2`, and `3`.
//
//     Note: `x=1&x=2&x=3` _also_ can also parse as `vec![1, 2, 3]` when viewed
//     as a `Vec<usize>`; this is the non-self-descriptive part of the format.
//
//   * `x[]=1&x[]=2&x[]=3` parses as `vec![vec![1], vec![2], vec![3]]`
//
//     This proceeds nearly identically to the previous example, with the exception
//     that the top-level `Vec` sees the values `[]=1`, `[]=2`, and `[]=3`.
//
//   * `x[0]=1&x[0]=2&x[]=3` parses as `vec![vec![1, 2], vec![3]]`
//
//     The top-level `Vec` sees the values `[0]=1`, `[0]=2`, and `[]=3`. The first
//     value results in a new `Vec<usize>` being initialized, as before, which is
//     pushed a `1`. The second value has the same index as the first, `0`, and so
//     `2` is pushed to the previous `T`, the `Vec` which contains the `1`.
//     Finally, the third value has no index, so a new `Vec<usize>` is initialized
//     and pushed a `3`.
//
//   * `x[0]=1&x[0]=2&x[]=3&x[]=4` parses as `vec![vec![1, 2], vec![3], vec![4]]`
//   * `x[0]=1&x[0]=2&x[1]=3&x[1]=4` parses as `vec![vec![1, 2], vec![3, 4]]`
//
// The indexing kind `[]` is purely by convention: the first two examples are
// equivalent to `x.=1&x.=2`, while the third to `x.0=1&x.0=&x.=3`.
//
// The parser proceeds as follows:
//
//   1. **Initialization.** The context stores parsing options, the
//      `last_index` encountered in a `push`, an `Option` of a `T::Context` for
//      the `current` value being parsed, a `Vec<T::Errors>` of `errors`, and
//      finally a `Vec<T>` of already parsed `items`.
//
//      ```rust,ignore
//      struct VecContext<'v, T: FromForm<'v>> {
//          opts: FormOptions,
//          last_index: Index<'v>,
//          current: Option<T::Context>,
//          errors: Vec<T::Error>,
//          items: Vec<T>
//      }
//      ```
//
//   2. **Push.** The index of the first key is compared against `last_index`.
//      If it differs, a new context for `T` is created and the previous is
//      finalized. The `Ok` result from finalization is stored in `items` and
//      the `Err` in `errors`. Otherwise the `index` is the same, the `current`
//      context is retrieved, and the field stripped of the current key is
//      pushed to `T`. `last_index` is updated.
//
//      ```rust,ignore
//      fn push(this: &mut Self::Context, field: FormField<'v>) {
//          if this.last_index != field.index() {
//              this.shift(); // finalize `current`, add to `items`, `errors`
//              let mut context = T::init(this.opts);
//              T::push(&mut context, field.next());
//              this.current = Some(context);
//          } else {
//              let context = this.current.as_mut();
//              T::push(context, field.next())
//          }
//
//          this.last_index = field.index();
//      }
//      ```
//
//   3. **Finalization.** Any `current` context is finalized, storing the `Ok`
//      or `Err` as before. `Ok(items)` is returned if `errors` is empty,
//      otherwise `Err(errors)` is returned.
//
//      ```rust,ignore
//      fn finalize(mut this: Self::Context) -> Result<Self, Self::Error> {
//          this.shift(); // finalizes `current`, as before.
//          match this.errors.is_empty() {
//              true => Ok(this.items),
//              false => Err(this.errors)
//          }
//      }
//      ```
//
// ## Arbitrary Maps (`HashMap<K: FromForm, V: FromForm>`)
//
// A `HashMap<K, V>` can be parsed from keys with one index or, for composite
// key values, such as structures or sequences, multiple indices. We begin with
// a discussion of the simpler case: non-composite keys.
//
// ### Non-Composite Keys
//
// A non-composite value can be described by a single field with no indices.
// Strings and integers are examples of non-composite values. The push-parser
// for `HashMap<K, V>` for a non-composite `K` uses the index of the first key
// as the value of `K`; the remainder of the field is pushed to `V`'s parser:
//
//   1. **Initialization.** The context stores a column-based representation of
//      `keys` and `values`, a `key_map` from a string key to the column index,
//      an `errors` vector for storing errors as they arise, and the parsing
//      options.
//
//      ```rust,ignore
//      struct MapContext<'v, K: FromForm<'v>, V: FromForm<'v>> {
//          opts: FormOptions,
//          key_map: HashMap<&'v str, usize>,
//          keys: Vec<K::Context>,
//          values: Vec<V::Context>,
//          errors: Vec<MapError<'v, K::Error, V::Error>>,
//      }
//      ```
//
//   2. **Push.** The `key_map` index for the key associated with the index of
//      the first key in the field is retrieved. If such a key has not yet been
//      seen, a new key and value context are created, the key is pushed to
//      `K`'s parser, and the field minus the first key is pushed to `V`'s
//      parser.
//
//      ```rust,ignore
//      fn push(this: &mut Self::Context, field: FormField<'v>) {
//          let key = field.index();
//          let value_context = match this.key_map.get(Key) {
//              Some(i) => &mut this.values[i],
//              None => {
//                  let i = this.keys.len();
//                  this.key_map.insert(key, i);
//                  this.keys.push(K::init(this.opts));
//                  this.values.push(V::init(this.opts));
//                  K::push(&mut this.keys[i], key.into());
//                  &mut this.values[i]
//              }
//          };
//
//          V::push(value_context, field.next());
//      }
//      ```
//
//   3. **Finalization.** All key and value contexts are finalized; any errors
//      are collected in `errors`. If there are no errors, `keys` and `values`
//      are collected into a `HashMap` and returned. Otherwise, the errors are
//      returned.
//
//      ```rust,ignore
//      fn finalize(mut this: Self::Context) -> Result<Self, Self::Error> {
//          this.finalize_keys();
//          this.finalize_values();
//          if this.errors.is_empty() {
//              Ok(this.keys.into_iter().zip(this.values.into_iter()).collect())
//          } else {
//              Err(this.errors)
//          }
//      }
//      ```
//
// Examples of forms parseable via this parser are:
//
//   * `x[0].name=Bob&x[0].meows=true`as a `HashMap<usize, Cat>` parses with
//     `0` mapping to `Cat { name: "Bob", meows: true }`
//   * `x[0]name=Bob&x[0]meows=true`as a `HashMap<usize, Cat>` parses just as
//      above.
//   * `x[0]=Bob&x[0]=Sally&x[1]=Craig`as a `HashMap<usize, Vec<String>>`
//      just as `{ 0 => vec!["Bob", "Sally"], 1 => vec!["Craig"] }`.
//
// A `HashMap<K, V>` can be thought of as a vector of key-value pairs: `Vec<(K,
// V)` (row-based) or equivalently, as two vectors of keys and values: `Vec<K>`
// and `Vec<V>` (column-based). The implication is that indexing into a
// specific key or value requires _two_ indexes: the first to determine whether
// a key or value is being indexed to, and the second to determine _which_ key
// or value. The push-parser for maps thus optionally accepts two indexes for a
// single key to allow piece-by-piece build-up of arbitrary keys and values.
//
// The parser proceeds as follows:
//
//   1. **Initialization.** The context stores parsing options, a vector of
//      `key_contexts: Vec<K::Context>`, a vector of `value_contexts:
//      Vec<V::Context>`, a `mapping` from a string index to an integer index
//      into the `contexts`, and a vector of `errors`.
//   2. **Push.** An index is required; an error is emitted and `push` returns
//      if they field's first key does not contain an index. If the first key
//      contains _one_ index, a new `K::Context` and `V::Context` are created.
//      The key is pushed as the value to `K` and the remaining field as the
//      value to `V`. The key and value are finalized; if both succeed, the key
//      and value are stored in `keys` and `values`; otherwise the error(s) is
//      stored in `errors`.
//
//      If the first keys contains _two_ indices, the first must starts with
//      `k` or `v`, while the `second` is arbitrary. `mapping` is indexed by
//      `second`; the integer is retrieved. If none exists, new contexts are
//      created an added to `{key,value}_contexts`, and their index is mapped
//      to `second` in `mapping`. If the first index is `k`, the field,
//      stripped of the first key, is pushed to the key's context; the same is
//      done for the value's context is the first index is `v`.
//   3. **Finalization.** Every context is finalized; errors and `Ok` values
//      are collected. TODO: FINISH. Split this into two: one for single-index,
//      another for two-indices.

mod field;
mod options;
mod from_form;
mod from_form_field;
mod form;
mod context;
mod strict;
mod lenient;
mod parser;
mod buffer;
pub mod validate;
pub mod name;
pub mod error;

#[cfg(test)]
mod tests;

/// Type alias for `Result` with an error type of [`Errors`].
pub type Result<'v, T> = std::result::Result<T, Errors<'v>>;

#[doc(hidden)]
pub use rocket_codegen::{FromForm, FromFormField};

#[doc(inline)]
pub use self::error::{Errors, Error};

#[doc(hidden)]
pub use self::buffer::{SharedStack, Shareable};

pub use field::*;
pub use options::*;
pub use from_form_field::*;
pub use from_form::*;
pub use form::*;
pub use context::*;
pub use strict::*;
pub use lenient::*;

#[doc(hidden)]
pub mod prelude {
    pub use super::*;
    pub use super::name::*;
    pub use super::error::*;
}
