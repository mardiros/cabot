use url;


#[derive(Debug, Clone)]
pub enum CabotError {
    IOError(String),
    CertificateError(String),
    SchemeError(String),
    OpaqueUrlError(String),
    UrlParseError(url::ParseError),
    HttpResponseParseError(String),
    EncodingError(String),
}

pub type CabotResult<T> = Result<T, CabotError>;
