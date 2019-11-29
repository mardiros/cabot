//! Handle the decrypt TCP async via rust TLS and expose HTTP
//! TCP <=> TLS <-> HTTP
//! <=> is async
//! <-> is sync

use std::sync::Arc;
use std::pin::Pin;


use std::io::{Read as SyncRead, Write as SyncWrite};
use std::vec::Vec;

use async_std::prelude::*;
use async_std::io::{Read, Result as IoResult, Write};
use async_std::net::TcpStream;
use async_std::task::{Context, Poll};
use rustls::{ClientConfig, ClientSession, ProtocolVersion, Session};
use webpki::DNSNameRef;
use webpki_roots;

use super::errors::CabotError;
use super::results::CabotResult;

const BUFFER_PAGE_SIZE: usize = 4096;
// const RESPONSE_BUFFER_SIZE: usize = 1024;

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

// fn read_buf<T>(client: &mut T, buf: &mut [u8]) -> Vec<u8>
// where
//     T: SyncRead + Sized,
// {
//     let mut response: Vec<u8> = Vec::with_capacity(RESPONSE_BUFFER_SIZE);
//     while let Ok(count) = client.read(&mut buf[..]) {
//         if count > 0 {
//             response.extend_from_slice(&buf[0..count]);
//         } else {
//             break;
//         }
//     }
//     response
// }

pub struct TLSStream<'a> {
    tcpstream: &'a mut TcpStream,
    buf_tlsread: Vec<u8>,
    buf_tlswrite: Vec<u8>,
    buf_response: Vec<u8>,
    tlsclient: ClientSession,
}

impl<'a> TLSStream<'a> {
    pub fn new(tcpstream: &'a mut TcpStream, host: &str) -> CabotResult<Self> {
        Ok(TLSStream {
            tcpstream,
            buf_tlsread: Vec::new(),
            buf_tlswrite: Vec::new(),
            buf_response: Vec::new(),
            tlsclient: create_client(host)?,
        })
    }
    pub async fn handshake(&mut self) -> CabotResult<()> {
        let mut is_handshaking = true;
        while is_handshaking {
            while self.tlsclient.wants_write() {
                let count = self.tlsclient.write_tls(&mut self.buf_tlswrite).unwrap();
                debug!("Write {} TLS bytes", count);
                //debug!("{}", self.buf_tlswrite.len());
                self.tcpstream.write_all(&self.buf_tlswrite.as_slice()[0 .. count]).await?;
                self.tcpstream.flush().await?;
                self.buf_tlswrite.clear();
            }
            if self.tlsclient.wants_read() {
                let mut buf: [u8; BUFFER_PAGE_SIZE] = [0; BUFFER_PAGE_SIZE];
                let n = self.tcpstream.read(&mut buf[..]).await?;
                if n > 0 {
                    debug!("Read {} TCP bytes", n);
                    self.buf_tlsread.extend_from_slice(&buf[0 .. n]);
                    let count = self.tlsclient.read_tls(&mut self.buf_tlsread.as_slice())?;
                    debug!("Decode {} TLS bytes", count);

                    self.tlsclient.process_new_packets()?;

                    //let mut part: Vec<u8> = read_buf(&mut self.tlsclient, &mut self.buf_tlsread);
                    //self.buf_response.append(&mut part);
                    self.buf_tlsread.clear();
                    //self.buf_response.clear();
                }
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

        let mut read_len = 0;
        let mut read_len_tls: usize = 0;

        let mut buf_tlsread = [0; BUFFER_PAGE_SIZE];
        let n = futures_core::ready!(
            Pin::new(&mut self_.tcpstream).poll_read(cx, &mut buf_tlsread)
        );

        match n {
            Err(err) => return Poll::Ready(Err(err)),
            Ok(count) => {
                let slice = &buf_tlsread[..count];
                self_.buf_tlsread.extend_from_slice(&slice);
            }
        }

        let mut bufread = self_.buf_tlsread.as_slice();

        while self_.tlsclient.wants_read() {
            let count = self_.tlsclient.read_tls(&mut bufread)?;
            read_len_tls = read_len_tls + count;
            debug!("Read {} TLS bytes", count);
            if count == 0 {
                debug!("Break");
                break;
            }
        }


        self_.tlsclient.process_new_packets().unwrap();

        let mut bufr = [0; BUFFER_PAGE_SIZE];
        let ret = self_.tlsclient.read(&mut bufr[..]);
        if let Ok(count) = ret {
            read_len = read_len + count;
            if count > 0 {
                self_.buf_response.extend_from_slice(&buf[0..count]);
            }
        }
        Poll::Ready(Ok(read_len))
    }
}

impl<'a> Write for TLSStream<'a> {

    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<IoResult<usize>> {
        let self_ = Pin::get_mut(self);

        self_.tlsclient.write_all(&buf).unwrap();
        let stream = Pin::new(&mut *self_.tcpstream);

        while self_.tlsclient.wants_write() {
            let count = self_.tlsclient.write_tls(&mut self_.buf_tlswrite).unwrap();
            debug!("Write {} TLS bytes", count);
        }
        let n = futures_core::ready!(
            stream.poll_write(cx, self_.buf_tlswrite.as_slice())
        )?;
        self_.buf_tlswrite.clear();
        Poll::Ready(Ok(n))
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

