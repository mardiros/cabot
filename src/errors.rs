//! Define results and error. `Result<T, CabotError>`
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Error as IOError;
use std::string::FromUtf8Error;

use rustls::TLSError;
use url;

#[derive(Debug)]
/// Errors in cabot
pub enum CabotError {
    IOError(IOError),
    DNSLookupError(String),
    CertificateError(TLSError),
    SchemeError(String),
    HostnameParseError(String),
    OpaqueUrlError(String),
    UrlParseError(url::ParseError),
    HttpResponseParseError(String),
    EncodingError(FromUtf8Error),
}

impl Display for CabotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            CabotError::SchemeError(scheme) => format!("Unmanaged scheme: {}", scheme),
            CabotError::OpaqueUrlError(err) => format!("Opaque URL Error: {}", err),
            CabotError::HostnameParseError(name) => format!("Invalid hostname: {}", name),
            CabotError::UrlParseError(err) => format!("URL Parse Error: {}", err),
            CabotError::IOError(err) => format!("IOError: {:?}", err),
            CabotError::DNSLookupError(err) => format!("DNSLookupError: {}", err),
            CabotError::CertificateError(err) => format!("CertificateError: {}", err),
            // Unexpexcted Error, not used
            CabotError::HttpResponseParseError(_) => format!("Unexpected error"),
            CabotError::EncodingError(err) => format!("Cannot decode utf8: {}", err),
        };
        write!(f, "{:?}: {}", self, description)
    }
}

impl Error for CabotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
