//! Low level and internal http and https implementation.
use std::cmp;
use std::collections::HashMap;
use std::mem;
use std::time::Duration;
use std::vec::Vec;

use async_std::io::{self, stderr, Read, Result as IoResult, Write};
use async_std::net::{SocketAddr, TcpStream};
use async_std::prelude::*;
use log::Level::{Info, Warn};

use super::asynctls::TLSStream;
use super::constants;
use super::dns::Resolver;
use super::request::{Request, RequestBuilder};
use super::results::{CabotError, CabotResult};

/// How do we have to decode the http response.
#[derive(Debug, PartialEq)]
enum TransferEncoding {
    /// A content-lenght has been found in the header, we now how many bytes to read.
    ContentLength(usize),
    /// A header transfer-encoding chunked has been found, we will parce content length by chunk.
    Chunked,
    /// A header transfer-encoding header has been found but we don't provide an implementation for it.
    Unkown,
    /// The transfer encoding is not initialized yet.
    None,
}

/// Internal status used while decoding a chunked http resonse.
#[derive(Debug, PartialEq)]
enum TransferEncodingStatus {
    ReadingHeader,
    ReadingBody(usize),
}

/// 3xx implemented representations for redirection.
#[derive(Debug, PartialEq)]
enum HTTPRedirect {
    // unimplemented.
    //HTTPMultipleChoices(String),
    //HTTPNotModified(String),
    //HTTPUseProxy(String),
    /// 302 Found (temporary redirect)
    HTTPFound(String),
    /// 301 Moved Permanently
    HTTPMovedPermanently(String),
    /// 308 Permanent Redirect (like 301 but preserve the http method)
    HTTPPermanentRedirect(String),
    /// 303 See others
    HTTPSeeOther(String),
    /// 307 Temporary Redirect (like 307 but temporary)
    HTTPTemporaryRedirect(String),
}

/// Wraps errors to handle http redirections.
#[derive(Debug)]
enum RedirectError {
    CabotError(CabotError),
    IOError(io::Error),
    Redirect(HTTPRedirect),
}

/// Http redirections handler.
type RedirectResult<T> = Result<T, RedirectError>;

impl From<CabotError> for RedirectError {
    fn from(err: CabotError) -> RedirectError {
        RedirectError::CabotError(err)
    }
}

impl From<io::Error> for RedirectError {
    fn from(err: io::Error) -> RedirectError {
        RedirectError::IOError(err)
    }
}

/// drain the buffer safely, do not panic if the size is greater than the buffer.
fn drain_buffer<T>(buffer: &mut Vec<T>, size: usize) -> Vec<T> {
    if buffer.len() >= size {
        buffer.drain(size..).collect()
    } else {
        error!("invalid size in drained buffer");
        buffer.clear();
        buffer.drain(..).collect()
    }
}

impl From<&[u8]> for TransferEncoding {
    fn from(hdr: &[u8]) -> Self {
        let hdr = String::from_utf8_lossy(hdr);
        let hdrup = hdr.to_ascii_uppercase();
        match hdrup.as_str() {
            "CHUNKED" => TransferEncoding::Chunked,
            _ => TransferEncoding::Unkown,
        }
    }
}

/// HTTP Response decoder.
struct HttpDecoder<'a> {
    //// read the http response stream/
    reader: &'a mut (dyn Read + Unpin), // tls require Write
    /// write the decoded http response.
    /// note that the first write called contains the header
    /// then write response by chunked.
    writer: &'a mut (dyn Write + Unpin),
    /// internal buffer to decode the response.
    buffer: Vec<u8>,
    /// response decoding strategy
    transfer_encoding: TransferEncoding,
    /// internal state when decoding the response
    transfer_encoding_status: TransferEncodingStatus,
    /// max time to wait while reading chunks.
    read_timeout: Duration,
    /// status code
    status_code: [u8; 3],
}

impl<'a> HttpDecoder<'a> {
    /// Create the new http decoder.
    fn new(
        writer: &'a mut (dyn Write + Unpin),
        reader: &'a mut (dyn Read + Unpin),
        read_timeout: u64,
    ) -> Self {
        HttpDecoder {
            writer,
            reader,
            buffer: Vec::with_capacity(constants::BUFFER_PAGE_SIZE),
            transfer_encoding: TransferEncoding::None,
            transfer_encoding_status: TransferEncodingStatus::ReadingHeader,
            read_timeout: Duration::from_millis(read_timeout),
            status_code: b"000".to_owned(),
        }
    }

    /// extract the first line of the buffer in case there is some
    fn drain_line(&mut self) -> Option<Vec<u8>> {
        debug!("Drain line...");
        if let Some(pos) = self.buffer.iter().position(|&x| x == b'\n') {
            if pos > 0 && self.buffer.get(pos - 1) == Some(&b'\r') {
                let buffer = self.buffer.drain((pos + 1)..).collect();
                let res = mem::replace(&mut self.buffer, buffer);
                return Some(res);
            } else {
                warn!("Missing \\r");
                let buffer = self.buffer.drain((pos + 1)..).collect();
                let res = mem::replace(&mut self.buffer, buffer);
                return Some(res);
            }
        } else {
            debug!("Not \\n yet");
        }
        None
    }

    /// Read a chunk from the reader to the buffer.
    async fn chunk_read(&mut self) -> IoResult<usize> {
        let ret = io::timeout(self.read_timeout, async {
            let mut buf = [0; constants::BUFFER_PAGE_SIZE];
            let ret = self.reader.read(&mut buf[..]).await;
            if let Ok(count) = ret {
                if count > 0 {
                    self.buffer.extend_from_slice(&buf[..count]);
                }
            }
            ret
        });
        ret.await.map_err(|err| match err.kind() {
            io::ErrorKind::TimedOut => io::Error::new(err.kind(), "Read Timeout".to_owned()),
            _ => err,
        })
    }

    /// read http headers
    async fn read_status_line(&mut self) -> CabotResult<Vec<u8>> {
        info!("Reading status line...");
        loop {
            let _count = self.chunk_read().await?;
            if let Some(line) = self.drain_line() {
                if let Some(pos) = line.iter().position(|&x| x == b' ') {
                    if pos + 3 < line.len() {
                        self.status_code = [
                            line[pos + 1].clone(),
                            line[pos + 2].clone(),
                            line[pos + 3].clone(),
                        ];
                    }
                    // else error!
                } // else error
                return Ok(line);
            }
        }
    }

    /// read http headers
    async fn read_headers(&mut self) -> RedirectResult<()> {
        info!("Reading response headers...");
        let mut headers_buf = self.read_status_line().await?;
        info!("Reading response headers...");
        'outer: loop {
            while let Some(line) = self.drain_line() {
                self.process_header(line.as_slice())?; // a bit wrong, header can be multiline
                headers_buf.extend_from_slice(line.as_slice());
                debug!("line {}", String::from_utf8_lossy(line.as_slice()));
                if line.len() == 2 {
                    break 'outer; // CRLF
                }
            }
            let _count = self.chunk_read().await?;
        }
        self.writer.write(headers_buf.as_slice()).await.unwrap();
        Ok(())
    }
    fn process_transfer_encoding(&mut self, header_value: &str) {
        let tenc = header_value.trim();
        debug!("transfer encoding: {:?}", tenc);
        self.transfer_encoding = TransferEncoding::from(tenc.as_bytes());
    }

    fn process_content_length(&mut self, header_value: &str) {
        let clength = header_value.trim();
        debug!("content length: {:?}", clength);
        let clength = usize::from_str_radix(clength, 10).unwrap();
        self.transfer_encoding = TransferEncoding::ContentLength(clength);
    }

    fn process_location(&self, header_value: &str) -> RedirectResult<()> {
        let loc = header_value.trim().to_owned();
        debug!("location: {:?}", loc);
        match &self.status_code {
            b"301" => {
                return Err(RedirectError::Redirect(HTTPRedirect::HTTPMovedPermanently(
                    loc,
                )))
            }
            b"302" => return Err(RedirectError::Redirect(HTTPRedirect::HTTPFound(loc))),
            b"303" => return Err(RedirectError::Redirect(HTTPRedirect::HTTPSeeOther(loc))),
            b"307" => {
                return Err(RedirectError::Redirect(
                    HTTPRedirect::HTTPTemporaryRedirect(loc),
                ))
            }
            b"308" => {
                return Err(RedirectError::Redirect(
                    HTTPRedirect::HTTPPermanentRedirect(loc),
                ))
            }
            _ => Ok(()),
        }
    }

    fn process_header(&mut self, header: &[u8]) -> RedirectResult<()> {
        if let Some(pos) = header.iter().position(|&x| x == b':') {
            let header = String::from_utf8_lossy(header);
            let (key, val) = header.split_at(pos);
            let key = key.to_uppercase().replace("-", "_");
            let hdr = &val[1..];
            match key.as_str() {
                "TRANSFER_ENCODING" => {
                    self.process_transfer_encoding(hdr);
                }
                "CONTENT_LENGTH" => {
                    self.process_content_length(hdr);
                }
                "LOCATION" => {
                    if self.status_code[0] == b'3' {
                        self.process_location(hdr)?;
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    /// read the body, write to the given writer with when no strategy found
    async fn read_write_no_transfer_encoding(&mut self) -> IoResult<()> {
        loop {
            self.writer.write(self.buffer.as_slice()).await.unwrap();
            self.buffer.clear();
            let cnt = self.chunk_read().await?;
            if cnt == 0 {
                break;
            }
        }
        Ok(())
    }

    /// read the body, write to the given writer with when the strategy
    /// is based on the http header Content-Length.
    async fn read_content_length(&mut self, size: usize) -> IoResult<()> {
        let mut read_count = self.buffer.len();
        loop {
            self.writer.write(self.buffer.as_slice()).await.unwrap();
            self.buffer.clear();

            if read_count >= size {
                break;
            }
            read_count = read_count + self.chunk_read().await?;
            warn!("< {}", read_count);
        }
        Ok(())
    }

    /// read the body, write to the given writer with when the strategy
    /// is based on the http header Transfer-Encoding: chunked.
    async fn read_write_chunk(&mut self) -> IoResult<()> {
        loop {
            // we have data in the buffer while reading the headers
            let done = self.process_chunk().await?;
            if done {
                break;
            }
            let cnt = self.chunk_read().await?;
            if cnt == 0 {
                error!("No more chunk data to read");
                break;
            }
        }
        Ok(())
    }

    /// read the body, write to the given writer
    async fn stream_response(&mut self) -> IoResult<()> {
        info!("Reading body");
        match self.transfer_encoding {
            TransferEncoding::ContentLength(size) => {
                self.read_content_length(size).await?;
            }
            TransferEncoding::Chunked => {
                self.read_write_chunk().await?;
            }
            _ => {
                warn!("Neither Content-Length, not chunk is response header");
                self.read_write_no_transfer_encoding().await?;
            }
        }

        if self.buffer.len() > 0 {
            let b = String::from_utf8_lossy(self.buffer.as_slice());
            error!("Buffer not clear: {}", b);
        }

        self.writer.flush().await
    }

    /// Process the data in the buffer.
    async fn process_chunk(&mut self) -> IoResult<bool> {
        debug!(
            "transfer_encoding_status: {:?}",
            self.transfer_encoding_status
        );
        if self.buffer.len() == 0 {
            return Ok(false);
        }
        loop {
            if self.transfer_encoding_status == TransferEncodingStatus::ReadingHeader {
                debug!("Reading header in Transfer-Encoding chunked");
                // we read the chunk size to drain
                let header = self.drain_line();
                if header.is_none() {
                    break;
                }
                let header = header.unwrap();
                let size = String::from_utf8_lossy(header.as_slice());
                if size.len() > 2 {
                    let body_chunk_size =
                        usize::from_str_radix(size.trim(), 16).map_err(|_err| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Chunk part should be an hexa string",
                            )
                        })?;
                    if body_chunk_size == 0 {
                        error!("Reading last chars...");
                        self.drain_line();
                        return Ok(true);
                    }
                    self.transfer_encoding_status =
                        TransferEncodingStatus::ReadingBody(body_chunk_size);
                } else {
                    error!("Chunk Header has improper size: {:?}, {}", size, size.len());
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Chunk part header is empty, shoule be an hexa",
                    ));
                }
            }

            if let TransferEncodingStatus::ReadingBody(buf_size) = self.transfer_encoding_status {
                if buf_size >= constants::BUFFER_PAGE_SIZE {
                    let buf_size = buf_size - self.buffer.len();
                    self.writer.write(self.buffer.as_slice()).await?;
                    self.buffer.clear();
                    self.transfer_encoding_status = TransferEncodingStatus::ReadingBody(buf_size);
                    break;
                }
                if self.buffer.len() > (buf_size + 2) {
                    let mut buffer: Vec<u8> = self.buffer.drain(buf_size..).collect();
                    self.writer.write(self.buffer.as_slice()).await?;
                    self.buffer = drain_buffer(&mut buffer, 2); // CRLF
                    self.transfer_encoding_status = TransferEncodingStatus::ReadingHeader;
                    if self.buffer.len() < 4 {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return Ok(false);
    }
}

async fn log_req_line(line: &str, verbose: bool) {
    let line = format!("> {}", line);
    let line = line.trim_end();
    if log_enabled!(Info) {
        info!("{}", line);
    } else if verbose {
        writeln!(&mut stderr(), "{}", line).await.unwrap();
    }
}

/// log the request.
async fn log_request(request: &[u8], verbose: bool) {
    if !log_enabled!(Info) && !verbose {
        return;
    }
    let mut request = request;
    loop {
        if let Some(pos) = request.iter().position(|&x| x == b'\n') {
            let (line, req) = request.split_at(pos);
            let line = String::from_utf8_lossy(line);
            debug!("Pos {}, Len {}", pos, line.len());
            log_req_line(&line, verbose).await;
            request = &req[1..];
            if line.len() <= 2 {
                break;
            }
        }
    }
    let bodylen = request.len();
    if bodylen > 0 {
        let msg = format!("[{} bytes]", bodylen);
        log_req_line(msg.as_str(), verbose).await;
    }
}

/// Send the http request to the stream, and write the response back
/// to the out parameter.
async fn process_request(
    request: &Request,
    stream: &mut TcpStream,
    out: &mut (dyn Write + Unpin),
    verbose: bool,
    read_timeout: u64,
    request_timeout: u64,
    https: bool,
) -> RedirectResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose).await;

    let mut ownable_tls_client: Option<TLSStream>;
    let client: &mut (dyn Read + Unpin) = if https {
        let mut tls_client = TLSStream::new(stream, request.host())?;
        tls_client.starttls().await?;
        debug!("Sending request...");
        tls_client.write_all(&raw_request).await?;
        ownable_tls_client = Some(tls_client);
        ownable_tls_client.as_mut().unwrap()
    } else {
        debug!("Sending request...");
        stream.write_all(&raw_request).await?;
        stream
    };
    debug!("Request sent");
    debug!("Decoding response...");
    let mut http_decoder = HttpDecoder::new(out, client, read_timeout);
    http_decoder.read_headers().await?;

    if request_timeout > 0 {
        io::timeout(Duration::from_millis(request_timeout), async {
            http_decoder.stream_response().await
        })
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::TimedOut => {
                if err.to_string() == "Read Timeout" {
                    err
                } else {
                    io::Error::new(err.kind(), "Request Timeout".to_owned())
                }
            }
            _ => err,
        })?;
    } else {
        http_decoder.stream_response().await?;
    }
    Ok(())
}

/// Process the given http query, write response to the `out` writer.
pub async fn http_query(
    request: &Request,
    mut out: &mut (dyn Write + Unpin),
    authorities: &HashMap<String, SocketAddr>,
    verbose: bool,
    ipv4: bool,
    ipv6: bool,
    dns_timeout: u64,
    connect_timeout: u64,
    read_timeout: u64,
    request_timeout: u64,
    max_redir: u8,
) -> CabotResult<()> {
    debug!(
        "HTTP Query {} {}",
        request.http_method(),
        request.request_uri()
    );
    let mut redir_req: Option<Request>;
    let mut request = request;
    let mut followed_redir = max_redir;
    let read_timeout = if request_timeout > 0 {
        if verbose && read_timeout > request_timeout {
            writeln!(
                &mut stderr(),
                "* Read timeout is greater than request timeout, overridden ({}ms)",
                request_timeout,
            )
            .await
            .unwrap();
        }
        cmp::min(read_timeout, request_timeout)
    } else {
        read_timeout
    };
    let mut result: CabotResult<()> = Ok(());
    loop {
        let authority = request.authority();
        let addr = match authorities.get(authority) {
            Some(val) => {
                info!("Fetch authority {} using autorities map", authority);
                *val
            }
            None => {
                info!("Fetch authority {} using resolver", authority);
                let resolver = Resolver::new(verbose);
                resolver
                    .get_addr(authority, ipv4, ipv6, dns_timeout)
                    .await?
            }
        };

        info!("Connecting to {}", addr);
        let mut client = io::timeout(Duration::from_millis(connect_timeout), async {
            TcpStream::connect(addr).await
        })
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::TimedOut => io::Error::new(err.kind(), "Connection Timeout".to_owned()),
            _ => err,
        })?;

        let resp = process_request(
            request,
            &mut client,
            &mut out,
            verbose,
            read_timeout,
            request_timeout,
            match request.scheme() {
                "http" => false,
                "https" => true,
                _ => {
                    return Err(CabotError::SchemeError(format!(
                        "Unrecognized scheme {}",
                        request.scheme()
                    )))
                }
            },
        )
        .await;

        match resp {
            Err(RedirectError::Redirect(redir)) => {
                if followed_redir <= 0 {
                    if log_enabled!(Warn) {
                        warn!("Maximum redirects followed ({})", max_redir);
                    } else if verbose {
                        writeln!(
                            &mut stderr(),
                            "* Maximum redirects followed ({})",
                            max_redir
                        )
                        .await
                        .unwrap();
                    }
                    return Err(CabotError::MaxRedirectionAttempt(max_redir));
                }
                let mut redir_req_builder = match redir {
                    HTTPRedirect::HTTPMovedPermanently(url)
                    | HTTPRedirect::HTTPFound(url)
                    | HTTPRedirect::HTTPSeeOther(url) => RequestBuilder::new(url.as_str()),
                    HTTPRedirect::HTTPPermanentRedirect(url)
                    | HTTPRedirect::HTTPTemporaryRedirect(url) => {
                        let mut req = RequestBuilder::new(url.as_str())
                            .set_http_method(request.http_method());
                        if let Some(body) = request.body() {
                            req = req.set_body(body);
                        }
                        req
                    }
                };
                for header in request.headers() {
                    if header.to_ascii_uppercase().starts_with("USER-AGENT:") {
                        let (_, ua) = header.split_at(11);
                        redir_req_builder = redir_req_builder.set_user_agent(ua.trim());
                    } else if header.to_ascii_uppercase().starts_with("SET-COOKIE:") {
                        redir_req_builder = redir_req_builder.add_header(header);
                    }
                }
                redir_req = Some(redir_req_builder.build()?);
                request = redir_req.as_ref().unwrap();
                followed_redir = followed_redir - 1;
            }
            Err(RedirectError::IOError(err)) => {
                result = Err(CabotError::IOError(err));
                break;
            }
            Err(RedirectError::CabotError(err)) => {
                result = Err(err);
                break;
            }
            _ => break,
        }
    }
    out.flush().await.unwrap();
    result
}
