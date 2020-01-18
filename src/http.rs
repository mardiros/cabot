//! Low level and internal http and https implementation.

use std::collections::HashMap;
use std::time::Duration;

use async_std::io::{self, stderr, Read, Result as IoResult, Write};
use async_std::net::{SocketAddr, TcpStream};
use async_std::prelude::*;
use log::Level::Info;

use super::asynctls::TLSStream;
use super::constants;
use super::dns::Resolver;
use super::request::Request;
use super::results::{CabotError, CabotResult};

#[derive(Debug, PartialEq)]
enum TransferEncoding {
    ContentLength(usize),
    Chunked,
    Unkown,
    None,
}

#[derive(Debug, PartialEq)]
enum TransferEncodingStatus {
    ReadingHeader,
    ChunkHeader,
    ReadingBody(usize),
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

struct HttpDecoder<'a> {
    reader: &'a mut (dyn Read + Unpin), // tls require Write
    writer: &'a mut (dyn Write + Unpin),
    buffer: Vec<u8>,
    transfer_encoding: TransferEncoding,
    transfer_encoding_status: TransferEncodingStatus,
}

impl<'a> HttpDecoder<'a> {
    fn new(writer: &'a mut (dyn Write + Unpin), reader: &'a mut (dyn Read + Unpin)) -> Self {
        HttpDecoder {
            writer,
            reader,
            buffer: Vec::with_capacity(constants::BUFFER_PAGE_SIZE),
            transfer_encoding: TransferEncoding::None,
            transfer_encoding_status: TransferEncodingStatus::ReadingHeader,
        }
    }

    async fn chunk_read(&mut self) -> IoResult<usize> {
        let ret = io::timeout(Duration::from_secs(5), async {
            let mut buf = [0; constants::BUFFER_PAGE_SIZE];
            let ret = self.reader.read(&mut buf[..]).await;
            if let Ok(count) = ret {
                if count > 0 {
                    self.buffer.extend_from_slice(&buf[0..count]);
                }
            }
            ret
        });
        ret.await
    }
    async fn read_write_headers(&mut self) -> IoResult<()> {
        info!("Reading headers");
        loop {
            let _count = self.chunk_read().await?;
            if let Some(_) = self.process_headers().await {
                break;
            }
        }
        Ok(())
    }

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

    async fn read_write_chunk(&mut self) -> IoResult<()> {
        loop {
            let done = self.process_chunk().await?;
            if done {
                break;
            }
            let cnt = self.chunk_read().await?;
            if cnt == 0 {
                break;
            }
        }

        Ok(())
    }

    async fn read_write_body(&mut self) -> IoResult<()> {
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

        Ok(())
    }

    async fn read_write(&mut self) -> IoResult<()> {
        self.read_write_headers().await?;
        self.read_write_body().await?;
        self.writer.flush().await
    }

    async fn process_headers(&mut self) -> Option<usize> {
        let ret = {
            let resp_header: Vec<&[u8]> = constants::SPLIT_HEADERS_RE
                .splitn(self.buffer.as_slice(), 2)
                .collect();
            if resp_header.len() == 2 {
                // We have headers
                let headers = resp_header.get(0).unwrap();
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
                let resp = headers.len();
                self.writer.write(headers).await.unwrap();
                Some(resp + 4) // + CRLF CRLF
            } else {
                None
            }
        };
        if let Some(to_drain) = ret {
            let buffer = self.buffer.drain(to_drain..).collect();
            self.buffer = buffer;
            info!("End of Headers readched");
            debug!("Transfer encoding: {:?}", self.transfer_encoding);
            debug!("{:?}", String::from_utf8_lossy(self.buffer.as_slice()));
        }
        ret
    }
    async fn process_chunk(&mut self) -> IoResult<bool> {
        debug!(
            "transfer_encoding_status: {:?}",
            self.transfer_encoding_status
        );
        let mut can_process_buffer = self.buffer.len() > 0;
        let mut body_chunk_size = 0;
        let mut header_len: usize;
        while can_process_buffer {
            header_len = 0;
            if self.transfer_encoding_status == TransferEncodingStatus::ReadingHeader {
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
                            self.transfer_encoding_status = TransferEncodingStatus::ChunkHeader;
                        }
                    } else {
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
                let buffer: Vec<u8> = self.buffer.drain(header_len..).collect();
                self.buffer = buffer;
                debug!(
                    "After header cleanup: {}",
                    String::from_utf8_lossy(self.buffer.as_slice())
                );
                self.transfer_encoding_status =
                    TransferEncodingStatus::ReadingBody(body_chunk_size);
            }

            if let TransferEncodingStatus::ReadingBody(body_chunk_size) =
                self.transfer_encoding_status
            {
                error!("!! {} > {}", self.buffer.len(), body_chunk_size);
                if self.buffer.len() > body_chunk_size {
                    let mut buffer: Vec<u8> = self.buffer.drain(body_chunk_size..).collect();
                    self.writer.write(self.buffer.as_slice()).await?;

                    let buffer2 = buffer.drain(2..).collect(); // CRLF
                    self.buffer = buffer2;
                    self.transfer_encoding_status = TransferEncodingStatus::ReadingHeader;
                } else {
                    can_process_buffer = false;
                }
            } else {
                can_process_buffer = false;
            }
        }

        return Ok(false);
    }
}

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

async fn from_http(
    request: &Request,
    client: &mut TcpStream,
    out: &mut (dyn Write + Unpin),
    verbose: bool,
) -> CabotResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose).await;

    debug!("Sending request...");
    client.write_all(&raw_request).await?;

    debug!("Reading response headers...");

    let mut http_decoder = HttpDecoder::new(out, client);
    http_decoder.read_write().await?;
    Ok(())
}

async fn from_https(
    request: &Request,
    client: &mut TcpStream,
    out: &mut (dyn Write + Unpin),
    verbose: bool,
) -> CabotResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose).await;

    let mut tls_client = TLSStream::new(client, request.host())?;
    tls_client.starttls().await?;

    debug!("Sending request...");
    tls_client.write_all(&raw_request).await?;
    debug!("Request sent");

    debug!("Decoding response...");
    let mut http_decoder = HttpDecoder::new(out, &mut tls_client);
    http_decoder.read_write().await?;
    Ok(())
}

pub async fn http_query(
    request: &Request,
    mut out: &mut (dyn Write + Unpin),
    authorities: &HashMap<String, SocketAddr>,
    verbose: bool,
    ipv4: bool,
    ipv6: bool,
) -> CabotResult<()> {
    debug!(
        "HTTP Query {} {}",
        request.http_method(),
        request.request_uri()
    );

    let authority = request.authority();

    let addr = match authorities.get(authority) {
        Some(val) => {
            info!("Fetch authority {} using autorities map", authority);
            *val
        }
        None => {
            info!("Fetch authority {} using resolver", authority);
            let resolver = Resolver::new(verbose);
            resolver.get_addr(authority, ipv4, ipv6).await?
        }
    };

    info!("Connecting to {}", addr);
    let mut client = TcpStream::connect(addr).await?;

    // client.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    match request.scheme() {
        "http" => from_http(request, &mut client, &mut out, verbose).await?,
        "https" => from_https(request, &mut client, &mut out, verbose).await?,
        _ => {
            return Err(CabotError::SchemeError(format!(
                "Unrecognized scheme {}",
                request.scheme()
            )))
        }
    };

    out.flush().await.unwrap();

    Ok(())
}
