//! The HTTP Client that perform query

use std::io::{self, Write};
use std::fmt::Arguments;

use super::request::Request;
use super::http;
use super::response::{Response, ResponseBuilder};
use super::results::CabotResult;

/// Perform the http query
pub struct Client {
    verbose: bool
}


impl Client {

    /// Construct a new `Client`
    pub fn new() -> Self {
        Client{verbose: false}
    }

    /// Execute the query [Request](../request/struct.Request.html) and
    /// return the associate [Response](../response/struct.Response.html).
    pub fn execute(&self, request: &Request) -> CabotResult<Response> {
        let mut out = CabotLibWrite::new();
        http::http_query(&request, &mut out, self.verbose)?;
        out.response()
    }
}


struct CabotLibWrite {
    response_builder: ResponseBuilder
}


impl CabotLibWrite {

    pub fn new() -> Self {
        CabotLibWrite{response_builder: ResponseBuilder::new()}
    }

    pub fn response(&self) -> CabotResult<Response> {
        self.response_builder.build()
    }
}

impl Write for CabotLibWrite {


    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        info!("Parsing http response");
        let mut response = Vec::with_capacity(buf.len());
        response.extend_from_slice(buf);
        let response = unsafe { String::from_utf8_unchecked(response) };
        let response: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
        let header_len = response.get(0).unwrap().len();
        let mut headers: Vec<&str> = response.get(0).unwrap().split("\r\n").collect();
        let mut builder = ResponseBuilder::new();
        let status_line = headers.remove(0);
        debug!("Adding status line {}", status_line);
        builder = builder.set_status_line(status_line);
        for header in  headers.iter() {
            debug!("Adding header {}", header);
            builder = builder.add_header(header);
        }

        let body = if (header_len + 4) < buf.len() {
            &buf[(header_len + 4)..buf.len()]
        }
        else {
            &buf[..]
        }; 
        //debug!("Adding body {:?}", body);
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
        let response = vec!["HTTP/1.1 200 Ok",
                            "Content-Type: text/plain",
                            "Content-Length: 12",
                            "",
                            "Hello World!"];
        let response = response.join("\r\n");

        let mut out = CabotLibWrite::new();
        out.write_all(response.as_bytes()).unwrap();
        let response = out.response().unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        let headers: &[&str] = &["Content-Type: text/plain", "Content-Length: 12"];
        assert_eq!(response.headers(), headers);
        assert_eq!(response.body_as_string().unwrap(), "Hello World!".to_owned());

    }

}