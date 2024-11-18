use ref_cast::RefCast;

use crate::mtls::{x509, oid, bigint, Name, Result, Error};
use crate::request::{Request, FromRequest, Outcome};
use crate::http::Status;

/// A request guard for validated, verified client certificates.
///
/// This type is a wrapper over [`x509::TbsCertificate`] with convenient
/// methods and complete documentation. Should the data exposed by the inherent
/// methods not suffice, this type derefs to [`x509::TbsCertificate`].
///
/// # Request Guard
///
/// The request guard implementation succeeds if:
///
///   * MTLS is [configured](crate::mtls).
///   * The client presents certificates.
///   * The certificates are valid and not expired.
///   * The client's certificate chain was signed by the CA identified by the
///     configured `ca_certs` and with respect to SNI, if any. See [module level
///     docs](crate::mtls) for configuration details.
///
/// If the client does not present certificates, the guard _forwards_ with a
/// status of 401 Unauthorized.
///
/// If the certificate chain fails to validate or verify, the guard _fails_ with
/// the respective [`Error`] a status of 401 Unauthorized.
///
/// # Wrapping
///
/// To implement roles, the `Certificate` guard can be wrapped with a more
/// semantically meaningful type with extra validation. For example, if a
/// certificate with a specific serial number is known to belong to an
/// administrator, a `CertifiedAdmin` type can authorize as follow:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::mtls::{self, bigint::BigUint, Certificate};
/// use rocket::request::{Request, FromRequest, Outcome};
/// use rocket::outcome::try_outcome;
/// use rocket::http::Status;
///
/// // The serial number for the certificate issued to the admin.
/// const ADMIN_SERIAL: &str = "65828378108300243895479600452308786010218223563";
///
/// // A request guard that authenticates and authorizes an administrator.
/// struct CertifiedAdmin<'r>(Certificate<'r>);
///
/// #[rocket::async_trait]
/// impl<'r> FromRequest<'r> for CertifiedAdmin<'r> {
///     type Error = mtls::Error;
///
///     async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
///         let cert = try_outcome!(req.guard::<Certificate<'r>>().await);
///         if let Some(true) = cert.has_serial(ADMIN_SERIAL) {
///             Outcome::Success(CertifiedAdmin(cert))
///         } else {
///             Outcome::Forward(Status::Unauthorized)
///         }
///     }
/// }
///
/// #[get("/admin")]
/// fn admin(admin: CertifiedAdmin<'_>) {
///     // This handler can only execute if an admin is authenticated.
/// }
///
/// #[get("/admin", rank = 2)]
/// fn unauthorized(user: Option<Certificate<'_>>) {
///     // This handler always executes, whether there's a non-admin user that's
///     // authenticated (user = Some()) or not (user = None).
/// }
/// ```
///
/// # Example
///
/// To retrieve certificate data in a route, use `Certificate` as a guard:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::get;
/// use rocket::mtls::{self, Certificate};
///
/// #[get("/auth")]
/// fn auth(cert: Certificate<'_>) {
///     // This handler only runs when a valid certificate was presented.
/// }
///
/// #[get("/maybe")]
/// fn maybe_auth(cert: Option<Certificate<'_>>) {
///     // This handler runs even if no certificate was presented or an invalid
///     // certificate was presented.
/// }
///
/// #[get("/ok")]
/// fn ok_auth(cert: mtls::Result<Certificate<'_>>) {
///     // This handler does not run if a certificate was not presented but
///     // _does_ run if a valid (Ok) or invalid (Err) one was presented.
/// }
/// ```
#[derive(Debug, PartialEq)]
pub struct Certificate<'a> {
    x509: x509::X509Certificate<'a>,
    data: &'a CertificateDer<'a>,
}

pub use rustls::pki_types::CertificateDer;

#[crate::async_trait]
impl<'r> FromRequest<'r> for Certificate<'r> {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        use crate::outcome::{try_outcome, IntoOutcome};

        let certs: Outcome<_, Error> = req.connection
            .peer_certs
            .as_ref()
            .or_forward(Status::Unauthorized);

        let chain = try_outcome!(certs);
        Certificate::parse(chain.inner()).or_error(())
    }
}

impl<'a> Certificate<'a> {
    /// PRIVATE: For internal Rocket use only!
    fn parse<'r>(chain: &'r [CertificateDer<'r>]) -> Result<Certificate<'r>> {
        let data = chain.first().ok_or(Error::Empty)?;
        let x509 = Certificate::parse_one(data)?;
        Ok(Certificate { x509, data })
    }

    fn parse_one(raw: &[u8]) -> Result<x509::X509Certificate<'_>> {
        use oid::OID_X509_EXT_SUBJECT_ALT_NAME as SUBJECT_ALT_NAME;
        use x509::FromDer;

        let (left, x509) = x509::X509Certificate::from_der(raw)?;
        if !left.is_empty() {
            return Err(Error::Trailing(left.len()));
        }

        // Ensure we have a subject or a subjectAlt.
        if x509.subject().as_raw().is_empty() {
            if let Some(ext) = x509.extensions().iter().find(|e| e.oid == SUBJECT_ALT_NAME) {
                if let x509::ParsedExtension::SubjectAlternativeName(..) = ext.parsed_extension() {
                    return Err(Error::NoSubject);
                } else if !ext.critical {
                    return Err(Error::NonCriticalSubjectAlt);
                }
            } else {
                return Err(Error::NoSubject);
            }
        }

        Ok(x509)
    }

    #[inline(always)]
    fn inner(&self) -> &x509::TbsCertificate<'a> {
        &self.x509.tbs_certificate
    }

    /// Returns the serial number of the X.509 certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     let cert = cert.serial();
    /// }
    /// ```
    pub fn serial(&self) -> &bigint::BigUint {
        &self.inner().serial
    }

    /// Returns the version of the X.509 certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     let cert = cert.version();
    /// }
    /// ```
    pub fn version(&self) -> u32 {
        self.inner().version.0
    }

    /// Returns the subject (a "DN" or "Distinguished Name") of the X.509
    /// certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     if let Some(name) = cert.subject().common_name() {
    ///         println!("Hello, {}!", name);
    ///     }
    /// }
    /// ```
    pub fn subject(&self) -> &Name<'a> {
        Name::ref_cast(&self.inner().subject)
    }

    /// Returns the issuer (a "DN" or "Distinguished Name") of the X.509
    /// certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     if let Some(name) = cert.issuer().common_name() {
    ///         println!("Issued by: {}", name);
    ///     }
    /// }
    /// ```
    pub fn issuer(&self) -> &Name<'a> {
        Name::ref_cast(&self.inner().issuer)
    }

    /// Returns a slice of the extensions in the X.509 certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::{oid, x509, Certificate};
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     let subject_alt = cert.extensions().iter()
    ///         .find(|e| e.oid == oid::OID_X509_EXT_SUBJECT_ALT_NAME)
    ///         .and_then(|e| match e.parsed_extension() {
    ///             x509::ParsedExtension::SubjectAlternativeName(s) => Some(s),
    ///             _ => None
    ///         });
    ///
    ///     if let Some(subject_alt) = subject_alt {
    ///         for name in &subject_alt.general_names {
    ///             if let x509::GeneralName::RFC822Name(name) = name {
    ///                 println!("An email, perhaps? {}", name);
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn extensions(&self) -> &[x509::X509Extension<'a>] {
        self.inner().extensions()
    }

    /// Checks if the certificate has the serial number `number`.
    ///
    /// If `number` is not a valid unsigned integer in base 10, returns `None`.
    ///
    /// Otherwise, returns `Some(true)` if it does and `Some(false)` if it does
    /// not.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// const SERIAL: &str = "65828378108300243895479600452308786010218223563";
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     if cert.has_serial(SERIAL).unwrap_or(false) {
    ///         println!("certificate has the expected serial number");
    ///     }
    /// }
    /// ```
    pub fn has_serial(&self, number: &str) -> Option<bool> {
        let uint: bigint::BigUint = number.parse().ok()?;
        Some(&uint == self.serial())
    }

    /// Returns the raw, unmodified, DER-encoded X.509 certificate data bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # use rocket::get;
    /// use rocket::mtls::Certificate;
    ///
    /// const SHA256_FINGERPRINT: &str =
    ///     "CE C2 4E 01 00 FF F7 78 CB A4 AA CB D2 49 DD 09 \
    ///      02 EF 0E 9B DA 89 2A E4 0D F4 09 83 97 C1 97 0D";
    ///
    /// #[get("/auth")]
    /// fn auth(cert: Certificate<'_>) {
    ///     # fn sha256_fingerprint(bytes: &[u8]) -> String { todo!() }
    ///     if sha256_fingerprint(cert.as_bytes()) == SHA256_FINGERPRINT {
    ///         println!("certificate fingerprint matched");
    ///     }
    /// }
    /// ```
    pub fn as_bytes(&self) -> &'a [u8] {
        self.data
    }
}

impl<'a> std::ops::Deref for Certificate<'a> {
    type Target = x509::TbsCertificate<'a>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
