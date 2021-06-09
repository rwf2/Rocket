use serde::Serialize;
use indexmap::{IndexMap, IndexSet};

use crate::form::prelude::*;
use crate::http::Status;

/// An infallible form guard that records form fields and errors during parsing.
///
/// This form guard _never fails_. It should be use _only_ when the form
/// [`Context`] is required. In all other cases, prefer to use `T` directly.
///
/// # Usage
///
/// `Contextual` acts as a proxy for any form type, recording all submitted form
/// values and produced errors and associating them with their corresponding
/// field name. `Contextual` is particularly useful for rendering forms with
/// previously submitted values and errors associated with form input.
///
/// To retrieve the context for a form, use `Form<Contextual<'_, T>>` as a data
/// guard, where `T` implements `FromForm`. The `context` field contains the
/// form's [`Context`]:
///
/// ```rust
/// # use rocket::post;
/// # type T = String;
/// use rocket::form::{Form, Contextual};
///
/// #[post("/submit", data = "<form>")]
/// fn submit(form: Form<Contextual<'_, T>>) {
///     if let Some(ref value) = form.value {
///         // The form parsed successfully. `value` is the `T`.
///     }
///
///     // We can retrieve raw field values and errors.
///     let raw_id_value = form.context.field_value("id");
///     let id_errors = form.context.field_errors("id");
/// }
/// ```
///
/// `Context` serializes as a map, so it can be rendered in templates that
/// require `Serialize` types. See the [forms guide] for further usage details.
///
/// [forms guide]: https://rocket.rs/v0.5-rc/guide/requests/#context
#[derive(Debug)]
pub struct Contextual<'v, T> {
    /// The value, if it was successfully parsed, or `None` otherwise.
    pub value: Option<T>,
    /// The context with all submitted fields and associated values and errors.
    pub context: Context<'v>,
}

/// A form context containing received fields, values, and encountered errors.
///
/// A value of this type is produced by the [`Contextual`] form guard in its
/// [`context`](Contextual::context) field. `Context` contains an entry for
/// every form field submitted by the client regardless of whether the field
/// parsed or validated successfully.
///
/// # Field Values
///
/// The original, submitted field value(s) for a _value_ field can be retrieved
/// via [`Context::field_value()`] or [`Context::field_values()`]. Data fields do not have
/// their values recorded. All submitted field names, including data field
/// names, can be retrieved via [`Context::fields()`].
///
/// # Field Errors
///
/// # Serialization
///
/// When a value of this type is serialized, a `struct` or map with the
/// following fields is emitted:
///
/// | field         | type                               | description                          |
/// |---------------|------------------------------------|--------------------------------------|
/// | `errors`      | map: string to array of [`Error`]s | maps a field name to its errors      |
/// | `values`      | map: string to array of strings    | maps a field name to its form values |
/// | `data_fields` | array of strings                   | field names of all form data fields  |
/// | `form_errors` | array of [`Error`]s                | errors not associated with a field   |
///
/// See [`Error`](Error#serialization) for `Error` serialization details.
#[derive(Debug, Default, Serialize)]
pub struct Context<'v> {
    errors: IndexMap<NameBuf<'v>, Errors<'v>>,
    values: IndexMap<&'v Name, Vec<&'v str>>,
    data_fields: IndexSet<&'v Name>,
    form_errors: Errors<'v>,
    #[serde(skip)]
    status: Status,
}

impl<'v> Context<'v> {
    /// Returns the names of all submitted form fields, both _value_ and _data_
    /// fields.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     let field_names = form.context.fields();
    /// }
    /// ```
    pub fn fields(&self) -> impl Iterator<Item = &'v Name> + '_ {
        self.values.iter()
            .map(|(name, _)| *name)
            .chain(self.data_fields.iter().copied())
    }

    /// Returns the _first_ value, if any, submitted for the _value_ field named
    /// `name`.
    ///
    /// The type of `name` may be `&Name`, `&str`, or `&RawStr`. Lookup is
    /// case-sensitive but key-separator (`.` or `[]`) insensitive.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     let first_value_for_id = form.context.field_value("id");
    ///     let first_value_for_foo_bar = form.context.field_value("foo.bar");
    /// }
    /// ```
    pub fn field_value<N: AsRef<Name>>(&self, name: N) -> Option<&'v str> {
        self.values.get(name.as_ref())?.get(0).cloned()
    }

    /// Returns the values, if any, submitted for the _value_ field named
    /// `name`.
    ///
    /// The type of `name` may be `&Name`, `&str`, or `&RawStr`. Lookup is
    /// case-sensitive but key-separator (`.` or `[]`) insensitive.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     let values_for_id = form.context.field_values("id");
    ///     let values_for_foo_bar = form.context.field_values("foo.bar");
    /// }
    /// ```
    pub fn field_values<N>(&self, name: N) -> impl Iterator<Item = &'v str> + '_
        where N: AsRef<Name>
    {
        self.values
            .get(name.as_ref())
            .map(|e| e.iter().cloned())
            .into_iter()
            .flatten()
    }

    /// Returns an iterator over all of the errors in the context, including
    /// those not associated with any field.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     let errors = form.context.errors();
    /// }
    /// ```
    pub fn errors(&self) -> impl Iterator<Item = &Error<'v>> {
        self.errors.values()
            .map(|e| e.iter())
            .flatten()
            .chain(self.form_errors.iter())
    }

    /// Returns the errors associated with the field `name`. This method is
    /// roughly equivalent to:
    ///
    /// ```rust
    /// # use rocket::form::{Context, name::Name};
    /// # let context = Context::default();
    /// # let name = Name::new("foo");
    /// context.errors().filter(|e| e.is_for(name))
    /// # ;
    /// ```
    ///
    /// That is, it uses [`Error::is_for()`] to determine which errors are
    /// associated with the field named `name`. This considers all errors whose
    /// associated field name is a prefix of `name` to be an error for the field
    /// named `name`. In other words, it associates parent field errors with
    /// their children: `a.b`'s errors apply to `a.b.c`, `a.b.d` and so on but
    /// not `a.c`.
    ///
    /// Lookup is case-sensitive but key-separator (`.` or `[]`) insensitive.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     // Get all errors for field `id`.
    ///     let id = form.context.field_errors("id");
    ///
    ///     // Get all errors for `foo.bar` or `foo` if `foo` failed first.
    ///     let foo_bar = form.context.field_errors("foo.bar");
    /// }
    /// ```
    pub fn field_errors<'a, N>(&'a self, name: N) -> impl Iterator<Item = &Error<'v>> + '_
        where N: AsRef<Name> + 'a
    {
        self.errors.values()
            .map(|e| e.iter())
            .flatten()
            .filter(move |e| e.is_for(&name))
    }

    /// Returns the errors associated _exactly_ with the field `name`. Prefer
    /// [`Context::field_errors()`] instead.
    ///
    /// This method is roughly equivalent to:
    ///
    /// ```rust
    /// # use rocket::form::{Context, name::Name};
    /// # let context = Context::default();
    /// # let name = Name::new("foo");
    /// context.errors().filter(|e| e.is_for_exactly(name))
    /// # ;
    /// ```
    ///
    /// That is, it uses [`Error::is_for_exactly()`] to determine which errors
    /// are associated with the field named `name`. This considers _only_ errors
    /// whose associated field name is _exactly_ `name` to be an error for the
    /// field named `name`. This is _not_ what is typically desired as it
    /// ignores errors that occur in the parent which will result in missing
    /// errors associated with its chilren. Use [`Context::field_errors()`] in
    /// almost all cases.
    ///
    /// Lookup is case-sensitive but key-separator (`.` or `[]`) insensitive.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) {
    ///     // Get all errors for field `id`.
    ///     let id = form.context.exact_field_errors("id");
    ///
    ///     // Get all errors exactly for `foo.bar`. If `foo` failed, we will
    ///     // this will return no erorrs. Use `Context::field_errors()`.
    ///     let foo_bar = form.context.exact_field_errors("foo.bar");
    /// }
    /// ```
    pub fn exact_field_errors<'a, N>(&'a self, name: N) -> impl Iterator<Item = &Error<'v>> + '_
        where N: AsRef<Name> + 'a
    {
        self.errors.values()
            .map(|e| e.iter())
            .flatten()
            .filter(move |e| e.is_for_exactly(&name))
    }

    /// Returns the `max` of the statuses associated with all field errors.
    ///
    /// See [`Error::status()`] for details on how an error status is computed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::http::Status;
    /// use rocket::form::{Form, Contextual};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Contextual<'_, T>>) -> (Status, &'static str) {
    ///     (form.context.status(), "Thanks!")
    /// }
    /// ```
    pub fn status(&self) -> Status {
        self.status
    }

    /// Inject a single error `error` into the context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::http::Status;
    /// use rocket::form::{Form, Contextual, Error};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(mut form: Form<Contextual<'_, T>>) {
    ///     let error = Error::validation("a good error message")
    ///         .with_name("field_name")
    ///         .with_value("some field value");
    ///
    ///     form.context.push_error(error);
    /// }
    /// ```
    pub fn push_error(&mut self, error: Error<'v>) {
        self.status = std::cmp::max(self.status, error.status());
        match error.name {
            Some(ref name) => match self.errors.get_mut(name) {
                Some(errors) => errors.push(error),
                None => { self.errors.insert(name.clone(), error.into()); },
            }
            None => self.form_errors.push(error)
        }
    }

    /// Inject all of the errors in `errors` into the context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::post;
    /// # type T = String;
    /// use rocket::http::Status;
    /// use rocket::form::{Form, Contextual, Error};
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(mut form: Form<Contextual<'_, T>>) {
    ///     let error = Error::validation("a good error message")
    ///         .with_name("field_name")
    ///         .with_value("some field value");
    ///
    ///     form.context.push_errors(vec![error]);
    /// }
    /// ```
    pub fn push_errors<E: Into<Errors<'v>>>(&mut self, errors: E) {
        errors.into().into_iter().for_each(|e| self.push_error(e))
    }
}

impl<'f> From<Errors<'f>> for Context<'f> {
    fn from(errors: Errors<'f>) -> Self {
        let mut context = Context::default();
        context.push_errors(errors);
        context
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Contextual<'v, T> {
    type Context = (<T as FromForm<'v>>::Context, Context<'v>);

    fn init(opts: Options) -> Self::Context {
        (T::init(opts), Context::default())
    }

    fn push_value((ref mut val_ctxt, ctxt): &mut Self::Context, field: ValueField<'v>) {
        ctxt.values.entry(field.name.source()).or_default().push(field.value);
        T::push_value(val_ctxt, field);
    }

    async fn push_data((ref mut val_ctxt, ctxt): &mut Self::Context, field: DataField<'v, '_>) {
        ctxt.data_fields.insert(field.name.source());
        T::push_data(val_ctxt, field).await;
    }

    fn push_error((_, ref mut ctxt): &mut Self::Context, e: Error<'v>) {
        ctxt.push_error(e);
    }

    fn finalize((val_ctxt, mut context): Self::Context) -> Result<'v, Self> {
        let value = match T::finalize(val_ctxt) {
            Ok(value) => Some(value),
            Err(errors) => {
                context.push_errors(errors);
                None
            }
        };

        Ok(Contextual { value, context })
    }
}
