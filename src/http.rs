//! Low level and internal http and https implementation.

use std::collections::HashMap;
use std::io::{stderr, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use log::LogLevel::Info;
use rustls::{ClientConfig, ClientSession, ProtocolVersion, Session};
use webpki::DNSNameRef;
use webpki_roots;

use super::constants;
use super::dns::Resolver;
use super::request::Request;
use super::results::{CabotError, CabotResult};

const BUFFER_PAGE_SIZE: usize = 1024;
const RESPONSE_BUFFER_SIZE: usize = 1024;

fn log_request(request: &[u8], verbose: bool) {
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
            writeln!(&mut stderr(), "> {}", header).unwrap();
        }
        if bodylen > 0 {
            writeln!(&mut stderr(), "> [{} bytes]", bodylen).unwrap();
        }
        writeln!(&mut stderr(), ">").unwrap();
    }
}

fn read_buf<T>(client: &mut T, buf: &mut [u8]) -> Vec<u8>
where
    T: Read + Sized,
{
    let mut response: Vec<u8> = Vec::with_capacity(RESPONSE_BUFFER_SIZE);
    while let Ok(count) = client.read(&mut buf[..]) {
        if count > 0 {
            response.extend_from_slice(&buf[0..count]);
        } else {
            break;
        }
    }
    response
}

fn from_http(
    request: &Request,
    client: &mut TcpStream,
    out: &mut Write,
    verbose: bool,
) -> CabotResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    log_request(&raw_request, verbose);

    debug!("Sending request...");
    client.write_all(&raw_request).unwrap();
    let mut buf = [0; BUFFER_PAGE_SIZE];
    let response = read_buf(client, &mut buf);
    out.write_all(response.as_slice()).unwrap();
    Ok(())
}

fn from_https(
    request: &Request,
    mut client: &mut TcpStream,
    out: &mut Write,
    verbose: bool,
) -> CabotResult<()> {
    let request_bytes = request.to_bytes();
    let raw_request = request_bytes.as_slice();
    let mut response: Vec<u8> = Vec::with_capacity(RESPONSE_BUFFER_SIZE);
    let mut buf = [0; BUFFER_PAGE_SIZE];

    let mut config = ClientConfig::new();
    config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let rc_config = Arc::new(config);
    let host = DNSNameRef::try_from_ascii_str(request.host())
        .map_err(|_| CabotError::HostnameParseError(request.host().to_owned()))?;
    let mut tlsclient = ClientSession::new(&rc_config, host);
    let mut is_handshaking = true;
    loop {
        while tlsclient.wants_write() {
            let count = tlsclient.write_tls(&mut client).unwrap();
            debug!("Write {} TLS bytes", count);
        }
        if is_handshaking && !tlsclient.is_handshaking() {
            info!("Handshake complete");
            is_handshaking = false;
            let protocol = tlsclient.get_protocol_version();
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
            log_request(&raw_request, verbose);
            tlsclient.write_all(&raw_request).unwrap();
            let count = tlsclient.write_tls(&mut client).unwrap();
            debug!("Write {} TLS bytes", count);
        }

        if tlsclient.wants_read() {
            let count = tlsclient.read_tls(&mut client)?;
            debug!("Read {} TLS bytes", count);
            if count == 0 {
                break;
            }

            if let Err(err) = tlsclient.process_new_packets() {
                return Err(CabotError::CertificateError(err));
            }

            let mut part: Vec<u8> = read_buf(&mut tlsclient, &mut buf);
            response.append(&mut part);
        } else {
            break;
        }
    }
    out.write_all(response.as_slice()).unwrap();
    Ok(())
}

pub fn http_query(
    request: &Request,
    mut out: &mut Write,
    authorities: &HashMap<String, SocketAddr>,
    verbose: bool,
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
            resolver.get_addr(authority)?
        }
    };

    info!("Connecting to {}", addr);
    let mut client = TcpStream::connect(addr)?;

    client.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    match request.scheme() {
        "http" => from_http(request, &mut client, &mut out, verbose)?,
        "https" => from_https(request, &mut client, &mut out, verbose)?,
        _ => {
            return Err(CabotError::SchemeError(format!(
                "Unrecognized scheme {}",
                request.scheme()
            )))
        }
    };

    out.flush().unwrap();

    Ok(())
}
