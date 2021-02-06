use super::ClientCertificate;
use thiserror::Error;
use x509_parser::prelude::X509Error;

#[derive(Debug, Error)]
enum ErrorKind {
    #[error("failed to parse x509")]
    X509(#[from] nom::Err<X509Error>),
    #[error("data is not utf-8")]
    Utf8(#[from] X509Error)
}


/// Error for the [`Certificate::parse`](Certificate::parse) method.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct CertificateParseError(ErrorKind);

/// Basic certificate information
pub struct CertificateFields {
    /// All CN fields in subject information.
    pub common_names: Vec<String>,
    /// All OU fields in subject information.
    pub organisation_units: Vec<String>,
}

fn parse(crt: &[u8]) -> Result<CertificateFields, ErrorKind> {
    let crt =  x509_parser::parse_x509_certificate(&crt)?.1;
    let mut common_names = Vec::new();
    let mut organisation_units = Vec::new();

    let subject= crt.subject();
    for cn in subject.iter_common_name() {
        common_names.push(cn.as_str()?.to_string());
    }
    for ou in subject.iter_organizational_unit() {
        organisation_units.push(ou.as_str()?.to_string());
    }

    Ok(CertificateFields {
        common_names,
        organisation_units
    })
}

/*fn parse_attr_type_and_val(av: &AttributeTypeAndValue) -> Result<String, CertificateParseError> {

}*/



impl ClientCertificate {
    /// Parses several fields presented client certificate.
    ///
    /// This function returns error if it cannot parse certificate or cannot
    /// understand its content.
    ///
    /// For more advanced scenarios consider using crates such as
    /// `x509-parser` or `openssl`.
    pub fn parse(&self) -> Result<CertificateFields, CertificateParseError> {
        parse(&self.0.0).map_err(CertificateParseError)
    }
}