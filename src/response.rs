//! HTTP Response handling.
//! The TCP response stream is converted to a
//! [Response](../response/struct.Response.html) structure.
//!
//! # Example
//! ```
//! use cabot::response::ResponseBuilder;
//!
//! let response = ResponseBuilder::new()
//!     .set_status_line("HTTP/1.1 200 Ok")
//!     .add_header("Content-Type: application/json")
//!     .set_body(&[123, 125])
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(response.http_version(), "HTTP/1.1");
//! assert_eq!(response.status_code(), 200);
//! assert_eq!(response.status_line(), "200 Ok");
//! assert_eq!(response.headers(), &["Content-Type: application/json"]);
//! assert_eq!(response.body_as_string().unwrap(), "{}");
//! ```

use super::results::{CabotError, CabotResult};

/// Represent the parsed HTTP response.
#[derive(Debug)]
pub struct Response {
    http_version: String,
    status_code: usize,
    status_line: String,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl Response {
    fn new(
        http_version: String,
        status_code: usize,
        status_line: String,
        headers: Vec<String>,
        body: Option<Vec<u8>>,
    ) -> Response {
        Response {
            http_version,
            status_code,
            status_line,
            headers,
            body,
        }
    }

    /// The response http version such as `HTTP/1.1` extracted from the
    /// repsonse status line.
    pub fn http_version(&self) -> &str {
        self.http_version.as_str()
    }

    /// The status status code such as `200` extracted from the response status
    /// line.
    pub fn status_code(&self) -> usize {
        self.status_code
    }

    /// The status line such as `200 Ok`. The status line as defined in
    /// [rfc7230](https://tools.ietf.org/html/rfc7230#section-3.1.1) also
    /// contains the http version, but, for convenience, it has been stripped
    /// here but is available using the `http_version()` method.
    pub fn status_line(&self) -> &str {
        self.status_line.as_str()
    }

    /// Response headers.
    /// Headers are not key/value parsed here to avoid deduplicates them.
    /// But multiline headers
    /// ([obsolete line folding](https://tools.ietf.org/html/rfc7230#section-3.2]))
    /// are implemented as specified and CRLF separator are preserved.
    pub fn headers(&self) -> Vec<&str> {
        let headers: Vec<&str> = self.headers.iter().map(|s| s.as_ref()).collect();
        headers
    }

    /// Get the body in raw format.
    pub fn body(&self) -> Option<&[u8]> {
        match self.body {
            None => None,
            Some(ref body) => Some(body.as_slice()),
        }
    }

    /// Clone the body and retrieve it in a String object.
    ///
    /// Important: Currently assume the body is encoded in utf-8.
    ///
    /// Errors:
    ///
    ///  - CabotError::EncodingError in case the body is not an utf-8 string
    ///
    pub fn body_as_string(&self) -> CabotResult<String> {
        let body = match self.body {
            None => "".to_owned(),
            Some(ref body) => {
                let mut body_vec: Vec<u8> = Vec::new();
                body_vec.extend_from_slice(body);
                String::from_utf8(body_vec)?
            }
        };
        Ok(body)
    }
}

#[derive(Debug, Default)]
/// An internal class used to build response.
///
///
pub struct ResponseBuilder {
    status_line: Option<String>,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl ResponseBuilder {
    /// Construct a ResponseBuilder
    pub fn new() -> Self {
        ResponseBuilder {
            status_line: None,
            headers: Vec::new(),
            body: None,
        }
    }

    /// initialize the status line
    pub fn set_status_line(mut self, status_line: &str) -> Self {
        self.status_line = Some(status_line.to_string());
        self
    }

    /// Append an header
    pub fn add_header(mut self, header: &str) -> Self {
        self.headers.push(header.to_owned());
        self
    }

    /// Set a response body
    pub fn set_body(mut self, buf: &[u8]) -> Self {
        let mut body = Vec::with_capacity(buf.len());
        body.extend_from_slice(buf);
        self.body = Some(body);
        self
    }

    /// Build the Response with the initialized data.
    pub fn build(&self) -> CabotResult<Response> {
        let status_line = self
            .status_line
            .as_ref()
            .ok_or(CabotError::HttpResponseParseError(
                "No Status Line".to_owned(),
            ))?;

        let mut vec_status_line: Vec<&str> = status_line.splitn(3, ' ').collect();

        if vec_status_line.len() != 3 {
            return Err(CabotError::HttpResponseParseError(format!(
                "Malformed Status Line: {}",
                status_line
            )));
        }

        let http_version = vec_status_line.remove(0);
        if !http_version.starts_with("HTTP/") {
            return Err(CabotError::HttpResponseParseError(format!(
                "Unkown Protocol in Status \
                 Line: {}",
                status_line
            )));
        }

        let status_code = &vec_status_line[0];
        let status_code = status_code.parse().map_err(|_| {
            CabotError::HttpResponseParseError(format!("Malformed status code: {}", status_line))
        })?;
        let status_line = vec_status_line.as_slice().join(" ");

        Ok(Response::new(
            http_version.to_owned(),
            status_code,
            status_line,
            self.headers.to_owned(),
            self.body.to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_ok() {
        let response = Response::new(
            "HTTP/1.1".to_owned(),
            200,
            "200 Ok".to_owned(),
            vec!["Content-Type: application/json".to_owned()],
            Some(vec![123, 125]),
        );

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        assert_eq!(response.headers(), &["Content-Type: application/json"]);
        let body: &[u8] = &[123, 125];
        assert_eq!(response.body(), Some(body));
        assert_eq!(response.body_as_string().unwrap(), "{}");
    }

    #[test]
    fn test_response_ok_no_body() {
        let response = Response::new(
            "HTTP/1.1".to_owned(),
            204,
            "204 No Content".to_owned(),
            vec![],
            None,
        );

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 204);
        assert_eq!(response.status_line(), "204 No Content");
        let headers: &[&str] = &[];
        assert_eq!(response.headers(), headers);
        assert_eq!(response.body(), None);
        assert_eq!(response.body_as_string().unwrap(), "".to_string());
    }

    #[test]
    fn test_build_response_ok() {
        let response = ResponseBuilder::new()
            .set_status_line("HTTP/1.1 200 Ok")
            .add_header("Content-Type: application/json")
            .set_body(&[123, 125])
            .build()
            .unwrap();

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        assert_eq!(response.headers(), &["Content-Type: application/json"]);
        let body: &[u8] = &[123, 125];
        assert_eq!(response.body(), Some(body));
        assert_eq!(response.body_as_string().unwrap(), "{}");
    }

}
