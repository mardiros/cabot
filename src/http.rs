//! Low level and internal http and https implementation.
use std::cmp;
use std::collections::HashMap;
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
    ChunkHeader,
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
        }
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
    async fn read_headers(&mut self) -> RedirectResult<()> {
        info!("Reading response headers...");
        loop {
            let _count = self.chunk_read().await?;
            let res = self.process_headers().await?;
            if let Some(_) = res {
                break;
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
                debug!("No more chunk data to read");
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

    /// Process headers to define the  body decoding strategy and
    /// to follow redirections.
    async fn process_headers(&mut self) -> RedirectResult<Option<usize>> {
        let ret = {
            let resp_header: Vec<&[u8]> = constants::SPLIT_HEADERS_RE
                .splitn(self.buffer.as_slice(), 2)
                .collect();
            if resp_header.len() == 2 {
                // We have the response headers

                let http_ver_headers = resp_header.get(0).unwrap().clone();
                let (_http_ver, headers) = http_ver_headers.split_at(9);
                if headers.starts_with(b"3") || headers.starts_with(b"3") {
                    let (code, _) = headers.split_at(3);
                    if let Some(header) = constants::LOCATION.captures(headers) {
                        if let Some(loc) = header.get(1) {
                            let loc = String::from_utf8_lossy(loc.as_bytes()).into_owned();
                            match code {
                                b"301" => {
                                    return Err(RedirectError::Redirect(
                                        HTTPRedirect::HTTPMovedPermanently(loc),
                                    ))
                                }
                                b"302" => {
                                    return Err(RedirectError::Redirect(HTTPRedirect::HTTPFound(
                                        loc,
                                    )))
                                }
                                b"303" => {
                                    return Err(RedirectError::Redirect(
                                        HTTPRedirect::HTTPSeeOther(loc),
                                    ))
                                }
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
                                _ => (),
                            }
                        }
                    }
                }

                if let Some(header) = constants::TRANSFER_ENCODING.captures(headers) {
                    if let Some(tenc) = header.get(1) {
                        self.transfer_encoding = TransferEncoding::from(tenc.as_bytes());
                    }
                } else if let Some(header) = constants::CONTENT_LENGTH.captures(headers) {
                    if let Some(clength) = header.get(1) {
                        let clength = String::from_utf8_lossy(clength.as_bytes()).into_owned();
                        let clength = usize::from_str_radix(clength.as_str(), 10).unwrap();
                        self.transfer_encoding = TransferEncoding::ContentLength(clength);
                    }
                }
                let resp = http_ver_headers.len();
                self.writer.write(http_ver_headers).await.unwrap();
                Some(resp + 4) // + CRLF CRLF
            } else {
                None
            }
        };
        if let Some(to_drain) = ret {
            self.buffer = drain_buffer(&mut self.buffer, to_drain);
            info!("End of Headers reached");
            debug!("Transfer encoding: {:?}", self.transfer_encoding);
            debug!("{:?}", String::from_utf8_lossy(self.buffer.as_slice()));
        }
        Ok(ret)
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
        let mut body_chunk_size = 0;
        let mut header_len: usize;
        loop {
            header_len = 0;
            if self.transfer_encoding_status == TransferEncodingStatus::ReadingHeader {
                debug!("Reading header in Transfer-Encoding chunked");
                // we read the chunk size to drain
                let header: Vec<&[u8]> = constants::SPLIT_HEADER_BRE
                    .splitn(self.buffer.as_slice(), 2)
                    .collect();
                if header.len() == 2 {
                    if let Some(header) = constants::GET_CHUNK_SIZE.captures(header[0]) {
                        if let Some(size) = header.get(1) {
                            let size = size.as_bytes();
                            header_len = size.len() + 2;
                            let size = String::from_utf8_lossy(size).into_owned();
                            body_chunk_size = usize::from_str_radix(size.as_str(), 16).unwrap();
                            debug!("Chunk Size to read: {}", body_chunk_size);
                            self.transfer_encoding_status = TransferEncodingStatus::ChunkHeader;
                        } else {
                            error!("Chunk Header is invalid");
                        }
                    } else {
                        error!("Chunk Header has improper size");
                        // else return Error
                        // break;
                    }
                } else {
                    debug!(
                        "Chunked not complete: {}",
                        String::from_utf8_lossy(self.buffer.as_slice())
                    );
                }
            }

            if self.transfer_encoding_status == TransferEncodingStatus::ChunkHeader {
                if body_chunk_size == 0 {
                    debug!(
                        "0 chunked size received: {}",
                        String::from_utf8_lossy(self.buffer.as_slice())
                    );
                    self.buffer.clear(); // should we check that it is '0\r\n' ?
                    return Ok(true);
                }

                debug!(
                    "Before header cleanup: {}",
                    String::from_utf8_lossy(self.buffer.as_slice())
                );
                self.buffer = drain_buffer(&mut self.buffer, header_len);
                debug!(
                    "After header cleanup: {}",
                    String::from_utf8_lossy(self.buffer.as_slice())
                );
                self.transfer_encoding_status =
                    TransferEncodingStatus::ReadingBody(body_chunk_size);
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
                    body_chunk_size = 0;
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

/// log the request.
async fn log_request(request: &[u8], verbose: bool) {
    if !log_enabled!(Info) && !verbose {
        return;
    }
    let request: Vec<&[u8]> = constants::SPLIT_HEADERS_RE.splitn(request, 2).collect();
    let headers = String::from_utf8_lossy(&request[0]);
    let headers: Vec<&str> = constants::SPLIT_HEADER_RE.split(&headers).collect();
    let bodylen = if request.len() == 2 {
        let body = &request[1];
        body.len()
    } else {
        0
    };
    if log_enabled!(Info) {
        for header in headers {
            info!("> {}", header);
        }
        if bodylen > 0 {
            info!("> [{} bytes]", bodylen);
        }
        info!(">");
    } else if verbose {
        for header in headers {
            writeln!(&mut stderr(), "> {}", header).await.unwrap();
        }
        if bodylen > 0 {
            writeln!(&mut stderr(), "> [{} bytes]", bodylen)
                .await
                .unwrap();
        }
        writeln!(&mut stderr(), ">").await.unwrap();
    }
}

/// Read plain text http request (no encryption)
async fn from_http(
    request: &Request,
    client: &mut TcpStream,
    out: &mut (dyn Write + Unpin),
    verbose: bool,
    read_timeout: u64,
    request_timeout: u64,
) -> RedirectResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose).await;

    debug!("Sending request...");
    client.write_all(&raw_request).await?;

    let mut http_decoder = HttpDecoder::new(out, client, read_timeout);

    debug!("Reading response headers...");
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

/// Read TLS http request
async fn from_https(
    request: &Request,
    client: &mut TcpStream,
    out: &mut (dyn Write + Unpin),
    verbose: bool,
    read_timeout: u64,
    request_timeout: u64,
) -> RedirectResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose).await;

    let mut tls_client = TLSStream::new(client, request.host())?;
    tls_client.starttls().await?;

    debug!("Sending request...");
    tls_client.write_all(&raw_request).await?;
    debug!("Request sent");

    debug!("Decoding response...");
    let mut http_decoder = HttpDecoder::new(out, &mut tls_client, read_timeout);

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

        let resp = match request.scheme() {
            "http" => {
                from_http(
                    request,
                    &mut client,
                    &mut out,
                    verbose,
                    read_timeout,
                    request_timeout,
                )
                .await
            }
            "https" => {
                from_https(
                    request,
                    &mut client,
                    &mut out,
                    verbose,
                    read_timeout,
                    request_timeout,
                )
                .await
            }
            _ => {
                return Err(CabotError::SchemeError(format!(
                    "Unrecognized scheme {}",
                    request.scheme()
                )))
            }
        };
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
