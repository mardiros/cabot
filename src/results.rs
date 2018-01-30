//! Define results and error. `Result<T, CabotError>`

use url;


#[derive(Debug, Clone)]
/// Errors in cabot
pub enum CabotError {
    IOError(String),
    CertificateError(String),
    SchemeError(String),
    HostnameParseError(String),
    OpaqueUrlError(String),
    UrlParseError(url::ParseError),
    HttpResponseParseError(String),
    EncodingError(String),
}

/// Result used by method that can failed.
pub type CabotResult<T> = Result<T, CabotError>;
