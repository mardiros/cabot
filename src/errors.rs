//! Define results and error. `Result<T, CabotError>`
use std::error::Error;
use std::fmt::{self, Display};
use std::string::FromUtf8Error;

use async_std::io::Error as IOError;
use rustls::TLSError;
use url::ParseError as UrlParseError;

#[derive(Debug)]
/// Errors in cabot
pub enum CabotError {
    DNSLookupError(String),
    HostnameParseError(String),
    HttpResponseParseError(String),
    OpaqueUrlError(String),
    SchemeError(String),
    MaxRedirectionAttempt(u8),
    // Wrapped errors
    CertificateError(TLSError),
    EncodingError(FromUtf8Error),
    IOError(IOError),
    UrlParseError(UrlParseError),
}

impl Display for CabotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            CabotError::DNSLookupError(err) => format!("DNS Lookup Error: {}", err),
            CabotError::HostnameParseError(name) => format!("Invalid Hostname: {}", name),
            CabotError::HttpResponseParseError(err) => {
                format!("HTTP Response Parse Error: {}", err)
            }
            CabotError::OpaqueUrlError(url) => format!("Opaque URL Error: {}", url),
            CabotError::SchemeError(scheme) => format!("Unmanaged Scheme: {}", scheme),
            // Wrapped errors
            CabotError::CertificateError(err) => format!("Certificate Error: {}", err),
            CabotError::EncodingError(err) => format!("Utf8 Encoding Error: {}", err),
            CabotError::IOError(err) => format!("IO Error: {}", err),
            CabotError::UrlParseError(err) => format!("URL Parse Error: {}", err),
            CabotError::MaxRedirectionAttempt(max_redir) => {
                format!("Maximum redirection attempt: {}", max_redir)
            }
        };
        write!(f, "{}", description)
    }
}

impl Error for CabotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        let err: Option<&(dyn Error + 'static)> = match self {
            CabotError::CertificateError(err) => Some(err),
            CabotError::EncodingError(err) => Some(err),
            CabotError::IOError(err) => Some(err),
            CabotError::UrlParseError(err) => Some(err),
            _ => None,
        };
        err
    }
}

impl From<TLSError> for CabotError {
    fn from(err: TLSError) -> CabotError {
        CabotError::CertificateError(err)
    }
}

impl From<FromUtf8Error> for CabotError {
    fn from(err: FromUtf8Error) -> CabotError {
        CabotError::EncodingError(err)
    }
}

impl From<IOError> for CabotError {
    fn from(err: IOError) -> CabotError {
        CabotError::IOError(err)
    }
}

impl From<UrlParseError> for CabotError {
    fn from(err: UrlParseError) -> CabotError {
        CabotError::UrlParseError(err)
    }
}
