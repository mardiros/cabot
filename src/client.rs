//! The HTTP Client that perform query

use std::io::{self, Write};
use std::fmt::Arguments;
use std::collections::HashMap;
use std::net::SocketAddr;

use super::request::Request;
use super::http;
use super::response::{Response, ResponseBuilder};
use super::results::CabotResult;
use super::constants;

/// Perform the http query
#[derive(Default)]
pub struct Client {
    verbose: bool,
    authorities: HashMap<String, SocketAddr>,
}

impl Client {
    /// Construct a new `Client`
    pub fn new() -> Self {
        Client {
            verbose: false,
            authorities: HashMap::new(),
        }
    }

    /// Avoid DNS resolution, force an address to be resolve to a given endpoint.
    /// authority has the format "host:port".
    pub fn add_authority(&mut self, authority: &str, sock_addr: &SocketAddr) {
        self.authorities
            .insert(authority.to_owned(), sock_addr.clone());
    }

    /// Execute the query [Request](../request/struct.Request.html) and
    /// return the associate [Response](../response/struct.Response.html).
    pub fn execute(&self, request: &Request) -> CabotResult<Response> {
        let mut out = CabotLibWrite::new();
        http::http_query(&request, &mut out, &self.authorities, self.verbose)?;
        out.response()
    }
}

struct CabotLibWrite {
    response_builder: ResponseBuilder,
}

impl CabotLibWrite {
    pub fn new() -> Self {
        CabotLibWrite {
            response_builder: ResponseBuilder::new(),
        }
    }

    pub fn response(&self) -> CabotResult<Response> {
        self.response_builder.build()
    }
}

impl Write for CabotLibWrite {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        info!("Parsing http response");
        let response: Vec<&[u8]> = constants::SPLIT_HEADERS_RE.splitn(buf, 2).collect();
        let header_len = &response[0].len();
        let headers_str = String::from_utf8_lossy(&response[0]);
        let mut headers: Vec<&str> = constants::SPLIT_HEADER_RE.split(&headers_str).collect();

        let mut builder = ResponseBuilder::new();
        let status_line = headers.remove(0);
        debug!("Adding status line {}", status_line);
        builder = builder.set_status_line(status_line);
        let mut iter_header = headers.iter().peekable();
        let header = iter_header.next();
        if header.is_some() {
            let buf = header.unwrap();
            let mut header = String::with_capacity(buf.len() * 2);
            header.push_str(buf);
            loop {
                {
                    let buf = iter_header.peek();
                    if buf.is_none() {
                        builder = builder.add_header(header.as_str());
                        break;
                    }
                    let buf = buf.unwrap();
                    if buf.starts_with(' ') || buf.starts_with('\t') {
                        debug!("Obsolete line folded header reveived in {}", header);
                        header.push_str(" ");
                        header.push_str(buf.trim_left());
                    } else {
                        debug!("Adding header {}", header);
                        builder = builder.add_header(header.as_str());
                        header.clear();
                        header.push_str(buf);
                    }
                }
                let _ = iter_header.next();
            }
        }
        let body = if (header_len + 4) < buf.len() {
            &buf[(header_len + 4)..buf.len()]
        } else {
            &[]
        };
        // debug!("Adding body {:?}", body);
        builder = builder.set_body(body);
        self.response_builder = builder;
        // debug!("Response Builder - {:?}", self.response_builder);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    // Don't implemented unused method

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }

    fn write_fmt(&mut self, _: Arguments) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_http_response_from_string() {
        let response = vec![
            "HTTP/1.1 200 Ok",
            "Content-Type: text/plain",
            "Content-Length: 12",
            "",
            "Hello World!",
        ];
        let response = response.join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write_all(response.as_bytes()).unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        let headers: &[&str] = &["Content-Type: text/plain", "Content-Length: 12"];
        assert_eq!(response.headers(), headers);
        assert_eq!(
            response.body_as_string().unwrap(),
            "Hello World!".to_owned()
        );
    }

    #[test]
    fn test_build_http_header_obsolete_line_folding() {
        let response = vec![
            "HTTP/1.1 200 Ok",
            "ows: https://tools.ietf.org/html/rfc7230",
            "  #section-3.2.4",
            "Content-Length: 12",
            "",
            "Hello World!",
        ];
        let response = response.join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write_all(response.as_bytes()).unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        let headers: &[&str] = &[
            "ows: https://tools.ietf.org/html/rfc7230 #section-3.2.4",
            "Content-Length: 12",
        ];
        assert_eq!(response.headers(), headers);
        assert_eq!(
            response.body_as_string().unwrap(),
            "Hello World!".to_owned()
        );
    }

    #[test]
    fn test_build_http_header_obsolete_line_folding_tab() {
        let response = vec![
            "HTTP/1.1 200 Ok",
            "ows: https://tools.ietf.org/html/rfc7230",
            "\t#section-3.2.4",
            "Content-Length: 12",
            "",
            "Hello World!",
        ];
        let response = response.join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write_all(response.as_bytes()).unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        let headers: &[&str] = &[
            "ows: https://tools.ietf.org/html/rfc7230 #section-3.2.4",
            "Content-Length: 12",
        ];
        assert_eq!(response.headers(), headers);
        assert_eq!(
            response.body_as_string().unwrap(),
            "Hello World!".to_owned()
        );
    }

    #[test]
    fn test_build_http_no_response_body() {
        let response = vec![
            "HTTP/1.1 302 Moved",
            "Location: https://tools.ietf.org/html/rfc7230#section-3.3",
        ];
        let response = response.join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write_all(response.as_bytes()).unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 302);
        assert_eq!(response.status_line(), "302 Moved");
        let headers: &[&str] = &["Location: https://tools.ietf.org/html/rfc7230#section-3.3"];
        assert_eq!(response.headers(), headers);
        assert_eq!(response.body_as_string().unwrap(), "");
    }
}
