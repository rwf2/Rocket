use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::num::{
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
};

use time::{Date, Time, PrimitiveDateTime};
use time::{macros::format_description, format_description::FormatItem};

use crate::data::Capped;
use crate::http::uncased::AsUncased;
use crate::form::prelude::*;

/// Implied form guard ([`FromForm`]) for parsing a single form field.
///
/// Types that implement `FromFormField` automatically implement [`FromForm`]
/// via a blanket implementation. As such, all `FromFormField` types are form
/// guards and can appear as the type of values in derived `FromForm` struct
/// fields:
///
/// ```rust
/// # use rocket::form::FromForm;
/// #[derive(FromForm)]
/// struct Person<'r> {
///     name: &'r str,
///     age: u16
/// }
/// ```
///
/// # Deriving
///
/// `FromFormField` can be derived for C-like enums, where the generated
/// implementation case-insensitively parses fields with values equal to the
/// name of the variant or the value in `field(value = "...")`.
///
/// ```rust
/// # use rocket::form::FromFormField;
/// /// Fields with value `"simple"` parse as `Kind::Simple`. Fields with value
/// /// `"fancy"` parse as `Kind::SoFancy`.
/// #[derive(FromFormField)]
/// enum Kind {
///     Simple,
///     #[field(value = "fancy")]
///     SoFancy,
/// }
/// ```
///
/// # Provided Implementations
///
/// See [`FromForm`](crate::form::FromForm#provided-implementations) for a list
/// of all form guards, including those implemented via `FromFormField`.
///
/// # Implementing
///
/// Implementing `FromFormField` requires implementing one or both of
/// `from_value` or `from_data`, depending on whether the type can be parsed
/// from a value field (text) and/or streaming binary data. Typically, a value
/// can be parsed from either, either directly or by using request-local cache
/// as an intermediary, and parsing from both should be preferred when sensible.
///
/// `FromFormField` is an async trait, so implementations must be decorated with
/// an attribute of `#[rocket::async_trait]`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # struct MyType;
/// use rocket::form::{self, FromFormField, DataField, ValueField};
///
/// #[rocket::async_trait]
/// impl<'r> FromFormField<'r> for MyType {
///     fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
///         todo!("parse from a value or use default impl")
///     }
///
///     async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
///         todo!("parse from a value or use default impl")
///     }
/// }
/// ```
///
/// ## Example
///
/// The following example parses a custom `Person` type with the format
/// `$name:$data`, where `$name` is expected to be string and `data` is expected
/// to be any slice of bytes.
///
/// ```rust
/// # use rocket::post;
/// use rocket::data::ToByteUnit;
/// use rocket::form::{self, FromFormField, DataField, ValueField};
///
/// use memchr::memchr;
///
/// struct Person<'r> {
///     name: &'r str,
///     data: &'r [u8]
/// }
///
/// #[rocket::async_trait]
/// impl<'r> FromFormField<'r> for Person<'r> {
///     fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
///         match field.value.find(':') {
///             Some(i) => Ok(Person {
///                 name: &field.value[..i],
///                 data: field.value[(i + 1)..].as_bytes()
///             }),
///             None => Err(form::Error::validation("does not contain ':'"))?
///         }
///     }
///
///     async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
///         // Retrieve the configured data limit or use `256KiB` as default.
///         let limit = field.request.limits()
///             .get("person")
///             .unwrap_or(256.kibibytes());
///
///         // Read the capped data stream, returning a limit error as needed.
///         let bytes = field.data.open(limit).into_bytes().await?;
///         if !bytes.is_complete() {
///             Err((None, Some(limit)))?;
///         }
///
///         // Store the bytes in request-local cache and split at ':'.
///         let bytes = bytes.into_inner();
///         let bytes = rocket::request::local_cache!(field.request, bytes);
///         let (raw_name, data) = match memchr(b':', bytes) {
///             Some(i) => (&bytes[..i], &bytes[(i + 1)..]),
///             None => Err(form::Error::validation("does not contain ':'"))?
///         };
///
///         // Try to parse the name as UTF-8 or return an error if it fails.
///         let name = std::str::from_utf8(raw_name)?;
///         Ok(Person { name, data })
///     }
/// }
///
/// use rocket::form::{Form, FromForm};
///
/// // The type can be used directly, if only one field is expected...
/// #[post("/person", data = "<person>")]
/// fn person(person: Form<Person<'_>>) { /* ... */ }
///
/// // ...or as a named field in another form guard...
/// #[derive(FromForm)]
/// struct NewPerson<'r> {
///     person: Person<'r>
/// }
///
/// #[post("/person", data = "<person>")]
/// fn new_person(person: Form<NewPerson<'_>>) { /* ... */ }
/// ```
// NOTE: Ideally, we would have two traits instead one with two fallible
// methods: `FromFormValue` and `FromFormData`. This would be especially nice
// for use with query values, where `FromFormData` would make no sense.
//
// However, blanket implementations of `FromForm` for these traits would result
// in duplicate implementations of `FromForm`; we need specialization to resolve
// this concern. Thus, for now, we keep this as one trait.
#[crate::async_trait]
pub trait FromFormField<'v>: Send + Sized {
    /// Parse a value of `T` from a form value field.
    ///
    /// The default implementation returns an error of
    /// [`ValueField::unexpected()`].
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }

    /// Parse a value of `T` from a form data field.
    ///
    /// The default implementation returns an error of
    /// [`DataField::unexpected()`].
    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }

    /// Returns a default value, if any exists, to be used during lenient
    /// parsing when the form field is missing.
    ///
    /// A return value of `None` means that field is required to exist and parse
    /// successfully, always. A return value of `Some(default)` means that
    /// `default` should be used when a field is missing.
    ///
    /// The default implementation returns `None`.
    fn default() -> Option<Self> { None }
}

#[doc(hidden)]
pub struct FromFieldContext<'v, T: FromFormField<'v>> {
    field_name: Option<NameView<'v>>,
    field_value: Option<&'v str>,
    opts: Options,
    value: Option<Result<'v, T>>,
    pushes: usize
}

impl<'v, T: FromFormField<'v>> FromFieldContext<'v, T> {
    fn should_push(&mut self) -> bool {
        self.pushes += 1;
        self.value.is_none()
    }

    fn push(&mut self, name: NameView<'v>, result: Result<'v, T>) {
        fn is_unexpected(e: &Errors<'_>) -> bool {
            matches!(e.last().map(|e| &e.kind), Some(ErrorKind::Unexpected))
        }

        self.field_name = Some(name);
        match result {
            Err(e) if !self.opts.strict && is_unexpected(&e) => { /* ok */ },
            result => self.value = Some(result),
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromFormField<'v>> FromForm<'v> for T {
    type Context = FromFieldContext<'v, T>;

    fn init(opts: Options) -> Self::Context {
        FromFieldContext {
            opts,
            field_name: None,
            field_value: None,
            value: None,
            pushes: 0,
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        if ctxt.should_push() {
            ctxt.field_value = Some(field.value);
            ctxt.push(field.name, Self::from_value(field))
        }
    }

    async fn push_data(ctxt: &mut FromFieldContext<'v, T>, field: DataField<'v, '_>) {
        if ctxt.should_push() {
            ctxt.push(field.name, Self::from_data(field).await);
        }
    }

    fn finalize(ctxt: Self::Context) -> Result<'v, Self> {
        let mut errors = match ctxt.value {
            Some(Ok(val)) if !ctxt.opts.strict || ctxt.pushes <= 1 => return Ok(val),
            Some(Ok(_)) => Errors::from(ErrorKind::Duplicate),
            Some(Err(errors)) => errors,
            None if !ctxt.opts.strict => match <T as FromFormField>::default() {
                Some(default) => return Ok(default),
                None => Errors::from(ErrorKind::Missing)
            },
            None => Errors::from(ErrorKind::Missing),
        };

        if let Some(name) = ctxt.field_name {
            errors.set_name(name);
        }

        if let Some(value) = ctxt.field_value {
            errors.set_value(value);
        }

        Err(errors)
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<&'v str> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(Capped::from(field.value))
    }

    async fn from_data(f: DataField<'v, '_>) -> Result<'v, Self> {
        use crate::data::{Capped, Outcome, FromData};

        match <Capped<&'v str> as FromData>::from_data(f.request, f.data).await {
            Outcome::Success(p) => Ok(p),
            Outcome::Failure((_, e)) => Err(e)?,
            Outcome::Forward(..) => {
                Err(Error::from(ErrorKind::Unexpected).with_entity(Entity::DataField))?
            }
        }
    }
}

impl_strict_from_form_field_from_capped!(&'v str);

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<String> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(Capped::from(field.value.to_string()))
    }

    async fn from_data(f: DataField<'v, '_>) -> Result<'v, Self> {
        use crate::data::{Capped, Outcome, FromData};

        match <Capped<String> as FromData>::from_data(f.request, f.data).await {
            Outcome::Success(p) => Ok(p),
            Outcome::Failure((_, e)) => Err(e)?,
            Outcome::Forward(..) => {
                Err(Error::from(ErrorKind::Unexpected).with_entity(Entity::DataField))?
            }
        }
    }
}

impl_strict_from_form_field_from_capped!(String);

impl<'v> FromFormField<'v> for bool {
    fn default() -> Option<Self> {
        Some(false)
    }

    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        match field.value.as_uncased() {
            v if v == "off" || v == "no" || v == "false" => Ok(false),
            v if v.is_empty() || v == "on" || v == "yes" || v == "true" => Ok(true),
            // force a `ParseBoolError`
            _ => Ok("".parse()?),
        }
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<Cow<'v, str>> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let capped = <Capped<&'v str>>::from_value(field)?;
        Ok(capped.map(|s| s.into()))
    }

    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        let capped = <Capped<&'v str>>::from_data(field).await?;
        Ok(capped.map(|s| s.into()))
    }
}

impl_strict_from_form_field_from_capped!(Cow<'v, str>);

macro_rules! impl_with_parse {
    ($($T:ident),+ $(,)?) => ($(
        impl<'v> FromFormField<'v> for $T {
            #[inline(always)]
            fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
                Ok(field.value.parse()?)
            }
        }
    )+)
}

impl_with_parse!(
    f32, f64,
    isize, i8, i16, i32, i64, i128,
    usize, u8, u16, u32, u64, u128,
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
    Ipv4Addr, IpAddr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
);
//
// Keep formats in sync with 'FromFormField' impls.
static DATE_FMT: &[FormatItem<'_>] = format_description!("[year padding:none]-[month]-[day]");
static TIME_FMT1: &[FormatItem<'_>] = format_description!("[hour padding:none]:[minute]:[second]");
static TIME_FMT2: &[FormatItem<'_>] = format_description!("[hour padding:none]:[minute]");
static DATE_TIME_FMT1: &[FormatItem<'_>] =
    format_description!("[year padding:none]-[month]-[day]T[hour padding:none]:[minute]:[second]");
static DATE_TIME_FMT2: &[FormatItem<'_>] =
    format_description!("[year padding:none]-[month]-[day]T[hour padding:none]:[minute]");

impl<'v> FromFormField<'v> for Date {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let date = Self::parse(field.value, &DATE_FMT)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(date)
    }
}

impl<'v> FromFormField<'v> for Time {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let time = Self::parse(field.value, &TIME_FMT1)
            .or_else(|_| Self::parse(field.value, &TIME_FMT2))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(time)
    }
}

impl<'v> FromFormField<'v> for PrimitiveDateTime {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let dt = Self::parse(field.value, &DATE_TIME_FMT1)
            .or_else(|_| Self::parse(field.value, &DATE_TIME_FMT2))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(dt)
    }
}
