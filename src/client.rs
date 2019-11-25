//! The HTTP Client that perform query
use std::collections::HashMap;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;

use async_std::io::{self, Write};
use async_std::prelude::*;
use async_std::task::Context;
use async_std::task::Poll;

use super::constants;
use super::http;
use super::request::Request;
use super::response::{Response, ResponseBuilder};
use super::results::CabotResult;

/// Perform the http query
#[derive(Default)]
pub struct Client {
    verbose: bool,
    ipv4: bool,
    ipv6: bool,
    authorities: HashMap<String, SocketAddr>,
}

impl Client {
    /// Construct a new `Client`
    pub fn new() -> Self {
        Client {
            verbose: false,
            ipv4: true,
            ipv6: true,
            authorities: HashMap::new(),
        }
    }

    /// Set Address Type authorized for DNS resolution.
    /// If every version is set to false, the resolution will failed.
    pub fn set_ip_version(&mut self, ipv4: bool, ipv6: bool) {
        self.ipv4 = ipv4;
        self.ipv6 = ipv6;
    }

    /// Avoid DNS resolution, force an address to be resolve to a given endpoint.
    /// authority has the format "host:port".
    pub fn add_authority(&mut self, authority: &str, sock_addr: &SocketAddr) {
        self.authorities
            .insert(authority.to_owned(), sock_addr.clone());
    }

    /// Execute the query [Request](../request/struct.Request.html) and
    /// return the associate [Response](../response/struct.Response.html).
    pub async fn execute(&self, request: &Request) -> CabotResult<Response> {
        let mut out = CabotLibWrite::new();
        http::http_query(
            &request,
            &mut out,
            &self.authorities,
            self.verbose,
            self.ipv4,
            self.ipv6,
        )
        .await?;
        out.response()
    }
}

struct CabotLibWrite {
    header_read: bool,
    body_buffer: Vec<u8>,
    response_builder: ResponseBuilder,
}

impl CabotLibWrite {
    pub fn new() -> Self {
        CabotLibWrite {
            response_builder: ResponseBuilder::new(),
            body_buffer: Vec::new(),
            header_read: false,
        }
    }

    fn split_headers(&mut self, buf: &[u8]) {
        let headers = String::from_utf8_lossy(buf);
        let mut headers: Vec<&str> = constants::SPLIT_HEADER_RE.split(&headers).collect();

        let status_line = headers.remove(0);
        debug!("Adding status line {}", status_line);
        let builder = ResponseBuilder::new();
        let mut builder = builder.set_status_line(status_line);

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
                        header.push_str(buf.trim_start());
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
        self.response_builder = builder;
    }

    pub fn response(&self) -> CabotResult<Response> {
        self.response_builder.build()
    }
}

impl Write for CabotLibWrite {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let self_ = Pin::get_mut(self);
        if !self_.header_read {
            self_.split_headers(&buf);
            self_.header_read = true;
            Poll::Ready(Ok(0))
        } else {
            self_.body_buffer.extend_from_slice(&buf);
            Poll::Ready(Ok(buf.len()))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        let self_ = Pin::get_mut(self);
        info!("Adding body {:?}", self_.body_buffer);
        let builder = mem::replace(&mut self_.response_builder, ResponseBuilder::new());
        self_.response_builder = builder.set_body(self_.body_buffer.as_slice());
        Poll::Ready(Ok(()))
    }

    // Don't implemented unused method

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "Not Implemented")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std;

    #[async_std::test]
    async fn test_build_http_response_from_string() -> std::io::Result<()> {
        let response = [
            vec![
                "HTTP/1.1 200 Ok",
                "Content-Type: text/plain",
                "Content-Length: 12",
            ]
            .join("\r\n"),
            vec!["Hello World!"].join("\r\n"),
        ];

        let mut out = CabotLibWrite::new();
        out.write(response[0].as_bytes()).await.unwrap();
        out.write(response[1].as_bytes()).await.unwrap();
        out.flush().await.unwrap();
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
        Ok(())
    }

    #[async_std::test]
    async fn test_build_http_header_obsolete_line_folding() -> std::io::Result<()> {
        let response = [
            vec![
                "HTTP/1.1 200 Ok",
                "ows: https://tools.ietf.org/html/rfc7230",
                "  #section-3.2.4",
                "Content-Length: 12",
            ]
            .join("\r\n"),
            vec!["Hello World!"].join("\r\n"),
        ];

        let mut out = CabotLibWrite::new();
        out.write(response[0].as_bytes()).await.unwrap();
        out.write(response[1].as_bytes()).await.unwrap();
        out.flush().await.unwrap();
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
        Ok(())
    }

    #[async_std::test]
    async fn test_build_http_header_obsolete_line_folding_tab() -> std::io::Result<()> {
        let response = [
            vec![
                "HTTP/1.1 200 Ok",
                "ows: https://tools.ietf.org/html/rfc7230",
                "\t#section-3.2.4",
                "Content-Length: 12",
            ]
            .join("\r\n"),
            vec!["Hello World!"].join("\r\n"),
        ];

        let mut out = CabotLibWrite::new();
        out.write(response[0].as_bytes()).await.unwrap();
        out.write(response[1].as_bytes()).await.unwrap();
        out.flush().await.unwrap();
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
        Ok(())
    }

    #[async_std::test]
    async fn test_build_http_no_response_body() -> std::io::Result<()> {
        let response = vec![
            "HTTP/1.1 302 Moved",
            "Location: https://tools.ietf.org/html/rfc7230#section-3.3",
        ]
        .join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write(response.as_bytes()).await.unwrap();
        out.flush().await.unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 302);
        assert_eq!(response.status_line(), "302 Moved");
        let headers: &[&str] = &["Location: https://tools.ietf.org/html/rfc7230#section-3.3"];
        assert_eq!(response.headers(), headers);
        assert_eq!(response.body_as_string().unwrap(), "");
        Ok(())
    }
}
