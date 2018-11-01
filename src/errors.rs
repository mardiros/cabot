//! Define results and error. `Result<T, CabotError>`
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Error as IOError;
use std::string::FromUtf8Error;

use rustls::TLSError;
use url::ParseError as UrlParseError;

#[derive(Debug)]
/// Errors in cabot
pub enum CabotError {
    IOError(IOError),
    CertificateError(TLSError),
    UrlParseError(UrlParseError),
    DNSLookupError(String),
    SchemeError(String),
    HostnameParseError(String),
    OpaqueUrlError(String),
    HttpResponseParseError(String),
    EncodingError(FromUtf8Error),
}

impl Display for CabotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            CabotError::SchemeError(scheme) => format!("Unmanaged Scheme: {}", scheme),
            CabotError::OpaqueUrlError(url) => format!("Opaque URL Error: {}", url),
            CabotError::HostnameParseError(name) => format!("Invalid Hostname: {}", name),
            CabotError::UrlParseError(err) => format!("URL Parse Error: {}", err),
            CabotError::IOError(err) => format!("IO Error: {:?}", err),
            CabotError::DNSLookupError(err) => format!("DNS Lookup Error: {}", err),
            CabotError::CertificateError(err) => format!("Certificate Error: {}", err),
            // Unexpexcted Error, not used
            CabotError::HttpResponseParseError(err) => format!("HTTP Response Parse Error: {}", err),
            CabotError::EncodingError(err) => format!("Utf8 Encoding Error: {}", err),
        };
        write!(f, "{:?}: {}", self, description)
    }
}

impl Error for CabotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        let err: Option<&(dyn Error + 'static)> = match self {
            CabotError::UrlParseError(err) => Some(err),
            CabotError::IOError(err) => Some(err),
            CabotError::CertificateError(err) => Some(err),
            CabotError::EncodingError(err) => Some(err),
            _ => None
        };
        err
    }
}

impl From<IOError> for CabotError {
    fn from(err: IOError) -> CabotError {
        CabotError::IOError(err)
    }
}

impl From<TLSError> for CabotError {
    fn from(err: TLSError) -> CabotError {
        CabotError::CertificateError(err)
    }
}

impl From<UrlParseError> for CabotError {
    fn from(err: UrlParseError) -> CabotError {
        CabotError::UrlParseError(err)
    }
}
