use std::{io, mem};
use std::path::{PathBuf, Path};

use crate::Request;
use crate::http::{ContentType, Status};
use crate::data::{self, FromData, Data, Capped, N, Limits};
use crate::form::{FromFormField, ValueField, DataField, error::Errors};
use crate::outcome::IntoOutcome;
use crate::fs::FileName;

use tokio::task;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufRead, BufReader};
use tempfile::{NamedTempFile, TempPath};
use either::Either;

/// A data and form guard that streams data into a temporary file.
///
/// `TempFile` is a data and form field (both value and data fields) guard that
/// streams incoming data into file in a temporary location. The file is deleted
/// when the `TempFile` handle is dropped unless it is persisted with
/// [`TempFile::persist_to()`] or copied with [`TempFile::copy_to()`].
///
/// # Hazards
///
/// Temporary files are cleaned by system file cleaners periodically. While an
/// attempt is made not to delete temporary files in use, _detection_ of when a
/// temporary file is being used is unreliable. As a result, a time-of-check to
/// time-of-use race condition from the creation of a `TempFile` to the
/// persistence of the `TempFile` may occur. Specifically, the following
/// sequence may occur:
///
/// 1. A `TempFile` is created at random path `foo`.
/// 2. The system cleaner removes the file at path `foo`.
/// 3. Another application creates a file at path `foo`.
/// 4. The `TempFile`, ostensibly at path `foo`, is persisted unexpectedly
///    with contents different from those in step 1.
///
/// To safe-guard against this issue, you should ensure that your temporary file
/// cleaner, if any, does not delete files too eagerly.
///
/// # Configuration
///
/// `TempFile` is configured via the following [`config`](crate::config)
/// parameters:
///
/// | Name               | Default             | Description                             |
/// |--------------------|---------------------|-----------------------------------------|
/// | `temp_dir`         | [`env::temp_dir()`] | Directory for temporary file storage.   |
/// | `limits.file`      | 1MiB                | Default limit for all file extensions.  |
/// | `limits.file/$ext` | _N/A_               | Limit for files with extension `$ext`.  |
///
/// [`env::temp_dir()`]: std::env::temp_dir()
///
/// When used as a form guard, the extension `$ext` is identified by the form
/// field's `Content-Type` ([`ContentType::extension()`]). When used as a data
/// guard, the extension is identified by the Content-Type of the request, if
/// any. If there is no Content-Type, the limit `file` is used.
///
/// # Cappable
///
/// A data stream can be partially read into a `TempFile` even if the incoming
/// stream exceeds the data limit via the [`Capped<TempFile>`] data and form
/// guard.
///
/// # Examples
///
/// **Data Guard**
///
/// ```rust
/// # use rocket::post;
/// use rocket::fs::TempFile;
///
/// #[post("/upload", data = "<file>")]
/// async fn upload(mut file: TempFile<'_>) -> std::io::Result<()> {
///     file.persist_to("/tmp/complete/file.txt").await?;
///     Ok(())
/// }
/// ```
///
/// **Form Field**
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::fs::TempFile;
/// use rocket::form::Form;
///
/// #[derive(FromForm)]
/// struct Upload<'f> {
///     upload: TempFile<'f>
/// }
///
/// #[post("/form", data = "<form>")]
/// async fn upload(mut form: Form<Upload<'_>>) -> std::io::Result<()> {
///     form.upload.persist_to("/tmp/complete/file.txt").await?;
///     Ok(())
/// }
/// ```
///
/// See also the [`Capped`] documentation for an example of `Capped<TempFile>`
/// as a data guard.
#[derive(Debug)]
pub enum TempFile<'v> {
    #[doc(hidden)]
    File {
        file_name: Option<&'v FileName>,
        content_type: Option<ContentType>,
        path: Either<TempPath, PathBuf>,
        len: u64,
    },
    #[doc(hidden)]
    Buffered {
        content: &'v [u8],
    }
}

impl<'v> TempFile<'v> {
    /// Persists the temporary file, moving it to `path`. If a file exists at
    /// the target path, `self` will atomically replace it. `self.path()` is
    /// updated to `path`.
    ///
    /// This method _does not_ create a copy of `self`, nor a new link to the
    /// contents of `self`: it renames the temporary file to `path` and marks it
    /// as non-temporary. As a result, this method _cannot_ be used to create
    /// multiple copies of `self`. To create multiple links, use
    /// [`std::fs::hard_link()`] with `path` as the `src` _after_ calling this
    /// method.
    ///
    /// # Cross-Device Persistence
    ///
    /// Attempting to persist a temporary file across logical devices (or mount
    /// points) will result in an error. This is a limitation of the underlying
    /// OS. Your options are thus:
    ///
    ///   1. Store temporary file in the same logical device.
    ///
    ///      Change the `temp_dir` configuration parameter to be in the same
    ///      logical device as the permanent location. This is the preferred
    ///      solution.
    ///
    ///   2. Copy the temporary file using [`TempFile::copy_to()`] or
    ///      [`TempFile::move_copy_to()`] instead.
    ///
    ///      This is a _full copy_ of the file, creating a duplicate version of
    ///      the file at the destination. This should be avoided for performance
    ///      reasons.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) -> std::io::Result<()> {
    ///     # assert!(file.path().is_none());
    ///     # let some_path = std::env::temp_dir().join("some-persist.txt");
    ///     file.persist_to(&some_path).await?;
    ///     assert_eq!(file.path(), Some(&*some_path));
    ///
    ///     Ok(())
    /// }
    /// # let file = TempFile::Buffered { content: "hi".as_bytes() };
    /// # rocket::async_test(handle(file)).unwrap();
    /// ```
    pub async fn persist_to<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        let new_path = path.as_ref().to_path_buf();
        match self {
            TempFile::File { path: either, .. } => {
                let path = mem::replace(either, Either::Right(new_path.clone()));
                match path {
                    Either::Left(temp) => {
                        let result = task::spawn_blocking(move || temp.persist(new_path)).await
                            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "spawn_block"))?;

                        if let Err(e) = result {
                            *either = Either::Left(e.path);
                            return Err(e.error);
                        }
                    },
                    Either::Right(prev) => {
                        if let Err(e) = fs::rename(&prev, new_path).await {
                            *either = Either::Right(prev);
                            return Err(e);
                        }
                    }
                }
            }
            TempFile::Buffered { content } => {
                fs::write(&new_path, &content).await?;
                *self = TempFile::File {
                    file_name: None,
                    content_type: None,
                    path: Either::Right(new_path),
                    len: content.len() as u64
                };
            }
        }

        Ok(())
    }

    /// Persists the temporary file at its temporary path and creates a full
    /// copy at `path`. The `self.path()` is _not_ updated, unless no temporary
    /// file existed prior, and the temporary file is _not_ removed. Thus, there
    /// will be _two_ files with the same contents.
    ///
    /// Unlike [`TempFile::persist_to()`], this method does not incur
    /// cross-device limitations, at the performance cost of a full copy. Prefer
    /// to use `persist_to()` with a valid `temp_dir` configuration parameter if
    /// no more than one copy of a file is required.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) -> std::io::Result<()> {
    ///     # assert!(file.path().is_none());
    ///     # let some_path = std::env::temp_dir().join("some-file.txt");
    ///     file.copy_to(&some_path).await?;
    ///     # assert_eq!(file.path(), Some(&*some_path));
    ///     # let some_other_path = std::env::temp_dir().join("some-other.txt");
    ///     file.copy_to(&some_other_path).await?;
    ///     assert_eq!(file.path(), Some(&*some_path));
    ///     # assert_eq!(std::fs::read(some_path).unwrap(), b"hi");
    ///     # assert_eq!(std::fs::read(some_other_path).unwrap(), b"hi");
    ///
    ///     Ok(())
    /// }
    /// # let file = TempFile::Buffered { content: b"hi" };
    /// # rocket::async_test(handle(file)).unwrap();
    /// ```
    pub async fn copy_to<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        match self {
            TempFile::File { path: either, .. } => {
                let old_path = mem::replace(either, Either::Right(either.to_path_buf()));
                match old_path {
                    Either::Left(temp) => {
                        let result = task::spawn_blocking(move || temp.keep()).await
                            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "spawn_block"))?;

                        if let Err(e) = result {
                            *either = Either::Left(e.path);
                            return Err(e.error);
                        }
                    },
                    Either::Right(_) => { /* do nada */ }
                };

                tokio::fs::copy(&either, path).await?;
            }
            TempFile::Buffered { content } => {
                let path = path.as_ref();
                fs::write(&path, &content).await?;
                *self = TempFile::File {
                    file_name: None,
                    content_type: None,
                    path: Either::Right(path.to_path_buf()),
                    len: content.len() as u64
                };
            }
        }

        Ok(())
    }

    /// Persists the temporary file at its temporary path, creates a full copy
    /// at `path`, and then deletes the temporary file. `self.path()` is updated
    /// to `path`.
    ///
    /// Like [`TempFile::copy_to()`] and unlike [`TempFile::persist_to()`], this
    /// method does not incur cross-device limitations, at the performance cost
    /// of a full copy and file deletion. Prefer to use `persist_to()` with a
    /// valid `temp_dir` configuration parameter if no more than one copy of a
    /// file is required.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) -> std::io::Result<()> {
    ///     # assert!(file.path().is_none());
    ///     # let some_path = std::env::temp_dir().join("some-copy.txt");
    ///     file.move_copy_to(&some_path).await?;
    ///     # assert_eq!(file.path(), Some(&*some_path));
    ///
    ///     Ok(())
    /// }
    /// # let file = TempFile::Buffered { content: "hi".as_bytes() };
    /// # rocket::async_test(handle(file)).unwrap();
    /// ```
    pub async fn move_copy_to<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        let dest = path.as_ref();
        self.copy_to(dest).await?;

        if let TempFile::File { path, .. } = self {
            fs::remove_file(&path).await?;
            *path = Either::Right(dest.to_path_buf());
        }

        Ok(())
    }

    /// Open the file for reading, returning an `async` stream of the file.
    ///
    /// This method should be used sparingly. `TempFile` is intended to be used
    /// when the incoming data is destined to be stored on disk. If the incoming
    /// data is intended to be streamed elsewhere, prefer to implement a custom
    /// form guard via [`FromFormField`] that directly streams the incoming data
    /// to the ultimate destination.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    /// use rocket::tokio::io;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(file: TempFile<'_>) -> std::io::Result<()> {
    ///     let mut stream = file.open().await?;
    ///     io::copy(&mut stream, &mut io::stdout()).await?;
    ///     Ok(())
    /// }
    /// # let file = TempFile::Buffered { content: "hi".as_bytes() };
    /// # rocket::async_test(handle(file)).unwrap();
    /// ```
    pub async fn open(&self) -> io::Result<impl AsyncBufRead + '_> {
        use tokio_util::either::Either;

        match self {
            TempFile::File { path, .. } => {
                let path = match path {
                    either::Either::Left(p) => p.as_ref(),
                    either::Either::Right(p) => p.as_path(),
                };

                let reader = BufReader::new(File::open(path).await?);
                Ok(Either::Left(reader))
            },
            TempFile::Buffered { content } => {
                Ok(Either::Right(*content))
            },
        }
    }

    /// Returns the size, in bytes, of the file.
    ///
    /// This method does not perform any system calls.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// fn handler(file: TempFile<'_>) {
    ///     let file_len = file.len();
    /// }
    /// ```
    pub fn len(&self) -> u64 {
        match self {
            TempFile::File { len, .. } => *len,
            TempFile::Buffered { content } => content.len() as u64,
        }
    }

    /// Returns the path to the file if it is known.
    ///
    /// Once a file is persisted with [`TempFile::persist_to()`], this method is
    /// guaranteed to return `Some`. Prior to this point, however, this method
    /// may return `Some` or `None`, depending on whether the file is on disk or
    /// partially buffered in memory.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) -> std::io::Result<()> {
    ///     # assert!(file.path().is_none());
    ///     # let some_path = std::env::temp_dir().join("some-path.txt");
    ///     file.persist_to(&some_path).await?;
    ///     assert_eq!(file.path(), Some(&*some_path));
    ///     # assert_eq!(std::fs::read(some_path).unwrap(), b"hi");
    ///
    ///     Ok(())
    /// }
    /// # let file = TempFile::Buffered { content: b"hi" };
    /// # rocket::async_test(handle(file)).unwrap();
    /// ```
    pub fn path(&self) -> Option<&Path> {
        match self {
            TempFile::File { path: Either::Left(p), .. } => Some(p.as_ref()),
            TempFile::File { path: Either::Right(p), .. } => Some(p.as_path()),
            TempFile::Buffered { .. } => None,
        }
    }

    /// Returns the sanitized file name as specified in the form field.
    ///
    /// A multipart data form field can optionally specify the name of a file. A
    /// browser will typically send the actual name of a user's selected file in
    /// this field, but clients are also able to specify _any_ name, including
    /// invalid or dangerous file names. This method returns a sanitized version
    /// of that value, if it was specified, suitable and safe for use as a
    /// permanent file name.
    ///
    /// Note that you will likely want to prepend or append random or
    /// user-specific components to the name to avoid collisions; UUIDs make for
    /// a good "random" data.
    ///
    /// See [`FileName::as_str()`] for specifics on sanitization.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) -> std::io::Result<()> {
    ///     # let some_dir = std::env::temp_dir();
    ///     if let Some(name) = file.name() {
    ///         // Because of Rocket's sanitization, this is safe.
    ///         file.persist_to(&some_dir.join(name)).await?;
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn name(&self) -> Option<&str> {
        self.raw_name().and_then(|f| f.as_str())
    }

    /// Returns the raw name of the file as specified in the form field.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// async fn handle(mut file: TempFile<'_>) {
    ///     let raw_name = file.raw_name();
    /// }
    /// ```
    pub fn raw_name(&self) -> Option<&FileName> {
        match *self {
            TempFile::File { file_name, .. } => file_name,
            TempFile::Buffered { .. } => None
        }
    }

    /// Returns the Content-Type of the file as specified in the form field.
    ///
    /// A multipart data form field can optionally specify the content-type of a
    /// file. A browser will typically sniff the file's extension to set the
    /// content-type. This method returns that value, if it was specified.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::TempFile;
    ///
    /// #[post("/", data = "<file>")]
    /// fn handle(file: TempFile<'_>) {
    ///     let content_type = file.content_type();
    /// }
    /// ```
    pub fn content_type(&self) -> Option<&ContentType> {
        match self {
            TempFile::File { content_type, .. } => content_type.as_ref(),
            TempFile::Buffered { .. } => None
        }
    }

    async fn from<'a>(
        req: &Request<'_>,
        data: Data<'_>,
        file_name: Option<&'a FileName>,
        content_type: Option<ContentType>,
    ) -> io::Result<Capped<TempFile<'a>>> {
        let limit = content_type.as_ref()
            .and_then(|ct| ct.extension())
            .and_then(|ext| req.limits().find(&["file", ext.as_str()]))
            .or_else(|| req.limits().get("file"))
            .unwrap_or(Limits::FILE);

        let temp_dir = req.rocket().config().temp_dir.relative();
        let file = task::spawn_blocking(move || NamedTempFile::new_in(temp_dir));
        let file = file.await;
        let file = file.map_err(|_| io::Error::new(io::ErrorKind::Other, "spawn_block panic"))??;
        let (file, temp_path) = file.into_parts();

        let mut file = File::from_std(file);
        let fut = data.open(limit).stream_to(tokio::io::BufWriter::new(&mut file));
        let n = fut.await;
        let n = n?;
        let temp_file = TempFile::File {
            content_type, file_name,
            path: Either::Left(temp_path),
            len: n.written,
        };

        Ok(Capped::new(temp_file, n))
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<TempFile<'v>> {
    fn from_value(field: ValueField<'v>) -> Result<Self, Errors<'v>> {
        let n = N { written: field.value.len() as u64, complete: true  };
        Ok(Capped::new(TempFile::Buffered { content: field.value.as_bytes() }, n))
    }

    async fn from_data(
        f: DataField<'v, '_>
    ) -> Result<Self, Errors<'v>> {
        Ok(TempFile::from(f.request, f.data, f.file_name, Some(f.content_type)).await?)
    }
}

#[crate::async_trait]
impl<'r> FromData<'r> for Capped<TempFile<'_>> {
    type Error = io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        use yansi::Paint;

        let has_form = |ty: &ContentType| ty.is_form_data() || ty.is_form();
        if req.content_type().map_or(false, has_form) {
            let (tf, form) = ("TempFile<'_>".primary(), "Form<TempFile<'_>>".primary());
            warn_!("Request contains a form that will not be processed.");
            info_!("Bare `{}` data guard writes raw, unprocessed streams to disk.", tf);
            info_!("Did you mean to use `{}` instead?", form);
        }

        TempFile::from(req, data, None, req.content_type().cloned()).await
            .or_error(Status::BadRequest)
    }
}

impl_strict_from_form_field_from_capped!(TempFile<'v>);
impl_strict_from_data_from_capped!(TempFile<'_>);
