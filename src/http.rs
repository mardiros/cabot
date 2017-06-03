//! Low level and internal http and https implementation.

use std::sync::Arc;
use std::time::Duration;
use std::io::{Read, Write, stderr};
use std::net::TcpStream;

use rustls::{Session, ClientConfig, ClientSession, ProtocolVersion};
use webpki_roots;
use log::LogLevel::Info;

use super::request::Request;
use super::results::{CabotResult, CabotError};
use super::dns::Resolver;


const BUFFER_PAGE_SIZE: usize = 1024;
const RESPONSE_BUFFER_SIZE: usize = 1024;


fn log_request(request: &str, verbose: bool) {
    if !log_enabled!(Info) && !verbose {
        return;
    }
    let split = request.split("\r\n");
    if log_enabled!(Info) {
        for part in split {
            info!("> {}", part);
        }
    } else if verbose {
        for part in split {
            writeln!(&mut stderr(), "> {}", part).unwrap();
        }
    }
}


fn read_buf<T>(mut client: &mut T, mut buf: &mut [u8]) -> Vec<u8>
    where T: Read + Sized
{
    let mut response: Vec<u8> = Vec::with_capacity(RESPONSE_BUFFER_SIZE);
    loop {
        match client.read(&mut buf[..]) {
            Ok(count) => {
                if count > 0 {
                    response.extend_from_slice(&buf[0..count]);
                } else {
                    break;
                }
            }
            Err(_) => break, // connection is closed by client
        }
    }
    response
}


fn from_http(request: &Request,
             mut client: &mut TcpStream,
             mut out: &mut Write,
             verbose: bool)
             -> CabotResult<()> {

    let request_str = request.to_string();
    log_request(&request_str, verbose);

    debug!("Sending request {}", request_str);
    client.write(request_str.as_bytes()).unwrap();
    let mut buf = [0; BUFFER_PAGE_SIZE];
    let response = read_buf(client, &mut buf);
    out.write_all(response.as_slice()).unwrap();
    Ok(())
}

fn from_https(request: &Request,
              mut client: &mut TcpStream,
              mut out: &mut Write,
              verbose: bool)
              -> CabotResult<()> {

    let request_str = request.to_string();
    let mut response: Vec<u8> = Vec::with_capacity(RESPONSE_BUFFER_SIZE);
    let mut buf = [0; BUFFER_PAGE_SIZE];

    let mut config = ClientConfig::new();
    config.root_store.add_trust_anchors(&webpki_roots::ROOTS);
    let rc_config = Arc::new(config);
    let mut tlsclient = ClientSession::new(&rc_config, request.host());
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
            log_request(&request_str, verbose);
            tlsclient.write_all(request_str.as_bytes()).unwrap();
            let count = tlsclient.write_tls(&mut client).unwrap();
            debug!("Write {} TLS bytes", count);
        }

        if tlsclient.wants_read() {
            let count = tlsclient.read_tls(&mut client);
            if let Err(err) = count {
                error!("{:?}", err);
                return Err(CabotError::IOError(format!("{}", err)));
            }

            let count = count.unwrap();
            debug!("Read {} TLS bytes", count);
            if count == 0 {
                break;
            }

            if let Err(err) = tlsclient.process_new_packets() {
                return Err(CabotError::CertificateError(format!("{}", err)));
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


pub fn http_query(request: &Request, mut out: &mut Write, verbose: bool) -> CabotResult<()> {
    debug!("HTTP Query {} {}",
           request.http_method(),
           request.request_uri());

    let resolver = Resolver::new(verbose);
    let authority = request.authority();
    let addr = resolver.get_addr(authority);

    info!("Connecting to {}", addr);
    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    match request.scheme() {
        "http" => from_http(request, &mut client, &mut out, verbose)?,
        "https" => from_https(request, &mut client, &mut out, verbose)?,
        _ => {
            return Err(CabotError::SchemeError(format!("Unrecognized scheme {}", request.scheme())))
        }
    };

    out.flush().unwrap();

    Ok(())

}
