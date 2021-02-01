//! The HTTP Client that perform query
use std::collections::HashMap;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;

use async_std::io::{self, Write};
use async_std::task::Context;
use async_std::task::Poll;
use futures::future::{BoxFuture, Future};
use log::Level::Debug;

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
    read_timeout: u64,
    connect_timeout: u64,
    dns_timeout: u64,
    request_timeout: u64,
    max_redir: u8,
}

impl<'a> Client {
    /// Construct a new `Client`
    pub fn new() -> Self {
        Client {
            verbose: false,
            ipv4: true,
            ipv6: true,
            authorities: HashMap::new(),
            dns_timeout: constants::DNS_LOOKUP_TIMEOUT * 1000,
            connect_timeout: constants::CONNECT_TIMEOUT * 1000,
            read_timeout: constants::READ_TIMEOUT * 1000,
            request_timeout: constants::REQUEST_TIMEOUT * 1000,
            max_redir: constants::NUMBER_OF_REDIRECT,
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

    /// Set the timeout for DNS resolution in seconds.
    pub fn set_dns_timeout(&mut self, timeout: u64) {
        self.dns_timeout = timeout * 1000;
    }
    /// Set the connect socket timeout in seconds.
    pub fn set_connect_timeout(&mut self, timeout: u64) {
        self.connect_timeout = timeout * 1000;
    }
    /// Set the read socket timeout in seconds.
    pub fn set_read_timeout(&mut self, timeout: u64) {
        self.read_timeout = timeout * 1000;
    }
    /// Set the request timeout in seconds.
    pub fn set_request_timeout(&mut self, timeout: u64) {
        self.request_timeout = timeout * 1000;
    }

    /// Set the timeout for DNS resolution in milliseconds.
    pub fn set_dns_timeout_ms(&mut self, timeout: u64) {
        self.dns_timeout = timeout;
    }
    /// Set the connect socket timeout in milliseconds.
    pub fn set_connect_timeout_ms(&mut self, timeout: u64) {
        self.connect_timeout = timeout;
    }
    /// Set the read socket timeout in milliseconds.
    pub fn set_read_timeout_ms(&mut self, timeout: u64) {
        self.read_timeout = timeout;
    }

    /// Set the request timeout in milliseconds.
    pub fn set_request_timeout_ms(&mut self, timeout: u64) {
        self.request_timeout = timeout;
    }

    /// Set the number of redirection to follow before giving up and return it.
    pub fn set_max_redir(&mut self, max_redir: u8) {
        self.max_redir = max_redir;
    }

    /// Execute the [Request](../request/struct.Request.html) and
    /// return the associate [Response](../response/struct.Response.html).
    pub async fn execute(&self, request: &Request) -> CabotResult<Response> {
        self.execute_fut(request).await
    }

    /// Execute the [Request](../request/struct.Request.html) and
    /// return the associate [Response](../response/struct.Response.html) in a box
    /// in order to user it in a async-std task.
    pub fn execute_box(&'a self, request: &'a Request) -> BoxFuture<'a, CabotResult<Response>> {
        let fut = Box::pin(self.execute_fut(request));
        Box::pin(ResponseFuture { fut })
    }

    /// Execute the [Request](../request/struct.Request.html) and
    /// return a Future instance in order to use it Client public api.
    fn execute_fut(
        &'a self,
        request: &'a Request,
    ) -> impl Future<Output = CabotResult<Response>> + 'a {
        async move {
            let mut out = CabotLibWrite::new();
            http::http_query(
                request,
                &mut out,
                &self.authorities,
                self.verbose,
                self.ipv4,
                self.ipv6,
                self.dns_timeout,
                self.connect_timeout,
                self.read_timeout,
                self.request_timeout,
                self.max_redir,
            )
            .await?;
            out.response()
        }
    }
}

/// A Future that implement Send to use Client inside task::spwan
pub struct ResponseFuture<'a> {
    fut: Pin<Box<dyn Future<Output = CabotResult<Response>> + 'a>>,
}

unsafe impl<'a> Send for ResponseFuture<'a> {}

impl<'a> Future for ResponseFuture<'a> {
    type Output = CabotResult<Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.fut).poll(cx)
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
        let mut builder = ResponseBuilder::new();
        if let Some(pos) = buf.iter().position(|&x| x == b'\n') {
            let (status_line, hdrs) = buf.split_at(pos);
            let status_line = String::from_utf8_lossy(status_line);
            builder = builder.set_status_line(status_line.trim_end());
            let mut header = "".to_owned();
            for hdr in hdrs.split(|&x| x == b'\n') {
                let hdr = String::from_utf8_lossy(hdr);
                if hdr.starts_with(' ') || hdr.starts_with('\t') {
                    debug!("Obsolete line folded header reveived in {}", header);
                    header.push_str(" ");
                    header.push_str(hdr.trim());
                } else {
                    let clean_hdr = header.trim();
                    if clean_hdr.len() > 0 {
                        builder = builder.add_header(clean_hdr.trim());
                        header.clear();
                    }
                    header.push_str(hdr.trim());
                }
            }
            let clean_hdr = header.trim();
            if clean_hdr.len() > 0 {
                builder = builder.add_header(clean_hdr.trim());
            }
            self.response_builder = builder;
        }
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
        if log_enabled!(Debug) {
            let body = String::from_utf8_lossy(self_.body_buffer.as_slice());
            debug!("Adding body {:?}", body);
        }
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
    use async_std::prelude::*;

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
