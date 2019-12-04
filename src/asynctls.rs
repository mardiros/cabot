//! decrypt TLS data using rustls on a async-std TcpStream

use std::io::{Error as IoError, ErrorKind as IoErrorKind, Read as SyncRead, Write as SyncWrite};
use std::pin::Pin;
use std::sync::Arc;
use std::vec::Vec;

use async_std::io::{Read, Result as IoResult, Write};
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task::{Context, Poll};
use rustls::{ClientConfig, ClientSession, ProtocolVersion, Session};
use webpki::DNSNameRef;
use webpki_roots;

use super::constants;
use super::errors::CabotError;
use super::results::CabotResult;

fn create_config() -> Arc<ClientConfig> {
    let mut config = ClientConfig::new();
    config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let rc_config = Arc::new(config);
    rc_config
}

fn create_client(host: &str) -> CabotResult<ClientSession> {
    let rc_config = create_config();
    let host = DNSNameRef::try_from_ascii_str(host)
        .map_err(|_| CabotError::HostnameParseError(host.to_string()))?;
    let tlsclient = ClientSession::new(&rc_config, host);
    Ok(tlsclient)
}

/// Stream to read from a ciphered TcpStream and cipher data before
/// writing the that TcpStream.
/// Use it like a TcpStream, but call the starttls after calling new.
pub struct TLSStream<'a> {
    tcpstream: &'a mut TcpStream,
    tlsclient: ClientSession,
}

impl<'a> TLSStream<'a> {
    /// Create new TLSStream instance
    pub fn new(tcpstream: &'a mut TcpStream, host: &str) -> CabotResult<Self> {
        Ok(TLSStream {
            tcpstream,
            tlsclient: create_client(host)?,
        })
    }
    /// Call it just after creating the sream
    pub async fn starttls(&mut self) -> CabotResult<()> {
        let mut buf_tlswrite: Vec<u8> = Vec::new();
        let mut read_buf: [u8; constants::BUFFER_PAGE_SIZE] = [0; constants::BUFFER_PAGE_SIZE];
        let mut is_handshaking = true;
        while is_handshaking {
            while self.tlsclient.wants_write() {
                let count = self.write(&mut buf_tlswrite).await?;
                debug!("Write {} TLS bytes during hangshake", count);
            }
            if self.tlsclient.wants_read() {
                let count = self.read(&mut read_buf).await?;
                debug!("Read {} TLS bytes during hangshake", count);
            }
            if is_handshaking && !self.tlsclient.is_handshaking() {
                info!("Handshake complete");
                is_handshaking = false;
                let protocol = self.tlsclient.get_protocol_version();
                match protocol {
                    Some(ProtocolVersion::SSLv2) => {
                        info!("Protocol SSL v2 negociated");
                    }
                    Some(ProtocolVersion::SSLv3) => {
                        info!("Protocol SSL v3 negociated");
                    }
                    Some(ProtocolVersion::TLSv1_0) => {
                        info!("Protocol TLS v1.0 negociated");
                    }
                    Some(ProtocolVersion::TLSv1_1) => {
                        info!("Protocol TLS v1.1 negociated");
                    }
                    Some(ProtocolVersion::TLSv1_2) => {
                        info!("Protocol TLS v1.2 negociated");
                    }
                    Some(ProtocolVersion::TLSv1_3) => {
                        info!("Protocol TLS v1.3 negociated");
                    }
                    Some(ProtocolVersion::Unknown(num)) => {
                        info!("Unknown TLS Protocol negociated: {}", num);
                    }
                    None => {
                        info!("No TLS Protocol negociated");
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a> Read for TLSStream<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        let self_ = Pin::get_mut(self);

        let cnt = self_.tlsclient.read(&mut buf[..]).unwrap();
        if cnt > 0 {
            debug!("Remaining {} Unencrypted bytes", cnt);
            return Poll::Ready(Ok(cnt));
        }

        let mut tcp_buf: [u8; constants::BUFFER_PAGE_SIZE] = [0; constants::BUFFER_PAGE_SIZE];

        let count =
            futures_core::ready!(Pin::new(&mut self_.tcpstream).poll_read(cx, &mut tcp_buf[..]));

        match count {
            Err(err) => Poll::Ready(Err(err)),
            Ok(n) => {
                if n > 0 {
                    debug!("Read {} TCP bytes", n);
                    let count = self_.tlsclient.read_tls(&mut &tcp_buf[..n])?;
                    debug!("Decode {} TLS bytes", count);

                    let packets = self_.tlsclient.process_new_packets();
                    match packets {
                        Ok(_) => {
                            let cnt = self_.tlsclient.read(&mut buf[..]).unwrap();
                            debug!("Read {} Unencrypted bytes", cnt);
                            Poll::Ready(Ok(cnt))
                        }
                        Err(err) => Poll::Ready(Err(IoError::new(
                            IoErrorKind::InvalidData,
                            format!("{:?}", err),
                        ))),
                    }
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

impl<'a> Write for TLSStream<'a> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<IoResult<usize>> {
        let self_ = Pin::get_mut(self);
        let mut buf_tlswrite: Vec<u8> = Vec::with_capacity(constants::BUFFER_PAGE_SIZE);

        let ret = buf.len();

        self_.tlsclient.write_all(&buf)?;
        debug!("Write {} TLS Clear bytes", ret);
        while self_.tlsclient.wants_write() {
            let count = self_.tlsclient.write_tls(&mut buf_tlswrite)?;
            debug!("Write {} TLS Ciphered bytes", count);
        }
        let stream = Pin::new(&mut *self_.tcpstream);
        let count = futures_core::ready!(stream.poll_write(cx, buf_tlswrite.as_slice()))?;
        buf_tlswrite.clear();
        debug!("Write {} TLS Ciphered bytes", count);
        Poll::Ready(Ok(ret))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        let self_ = Pin::get_mut(self);
        Pin::new(&mut *self_.tcpstream).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        let self_ = Pin::get_mut(self);
        Pin::new(&mut *self_.tcpstream).poll_close(cx)
    }
}
