use std::sync::Arc;
use std::time::Duration;
use std::io::{self, Write, stderr};
use std::io::prelude::*;
use std::net::{TcpStream, ToSocketAddrs};

use rustls::{Session, ClientConfig, ClientSession, ProtocolVersion};
use webpki_roots;
use url::Url;
use log::LogLevel::Info;

use super::request::Request;
use super::results::{CabotResult, CabotError};
use super::dns::Resolver;

fn log_request(request: &str, verbose: bool) {
    let mut split = request.split("\r\n");
    if log_enabled!(Info) {
        for part in split {
            info!("> {}", part);
        }
    }
    else if verbose {
        for part in split {
            writeln!(&mut stderr(), "> {}", part).unwrap();
        }
    }
}

fn log_response(response: &str, length: usize, verbose: bool) {
    let mut split = response.split("\n");
    if log_enabled!(Info) {
        for part in split {
            info!("< {}", part);
        }
        info!("[[{} bytes data]]", length);
    }
    else if verbose {
        for part in split {
            writeln!(&mut stderr(), "< {}", part).unwrap();
        }
        writeln!(&mut stderr(), "< {}", length).unwrap();
    }
}


fn write_response(mut out: &mut Write,
                  response: String,
                  verbose: bool) {
    let mut response: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
    if response.len() == 2 {
        log_response(response.get(0).unwrap(),
                     response.get(1).unwrap().len(),
                     verbose);
    }
    out.write_all(response.get(1).unwrap().as_bytes());
}


pub fn from_http(request: &Request,
                 mut client: &mut TcpStream,
                 mut out: &mut Write,
                 verbose: bool)
                 -> CabotResult<()> {

    let request_str = request.to_string();
    log_request(&request_str, verbose);

    debug!("Sending request {}", request_str);
    client.write(request_str.as_bytes()).unwrap();

    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    write_response(out, response, verbose);
    Ok(())
}

pub fn from_https(request: &Request,
                  mut client: &mut TcpStream,
                  mut out: &mut Write,
                  verbose: bool)
                  -> CabotResult<()> {

    let request_str = request.to_string();

    let mut config = ClientConfig::new();
    config.root_store.add_trust_anchors(&webpki_roots::ROOTS);
    let rc_config = Arc::new(config);
    let mut tlsclient = ClientSession::new(&rc_config, request.host());
    let mut is_handshaking = true;
    let mut response = String::new();
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
                },
                Some(ProtocolVersion::SSLv3) => {
                    info!("Protocol SSL v3 negociated");
                },
                Some(ProtocolVersion::TLSv1_0) => {
                    info!("Protocol TLS v1.0 negociated");
                },
                Some(ProtocolVersion::TLSv1_1) => {
                    info!("Protocol TLS v1.1 negociated");
                },
                Some(ProtocolVersion::TLSv1_2) => {
                    info!("Protocol TLS v1.2 negociated");
                },
                Some(ProtocolVersion::TLSv1_3) => {
                    info!("Protocol TLS v1.3 negociated");
                },
                Some(ProtocolVersion::Unknown(num)) => {
                    info!("Unknown TLS Protocol negociated: {}", num);
                },
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

            let mut part: Vec<u8> = Vec::new();
            let clearcount = tlsclient.read_to_end(&mut part);
            if let Err(err) = clearcount {
                let spart = unsafe {
                    String::from_utf8_unchecked(part)
                };
                response.push_str(spart.as_str());

                if err.kind() == io::ErrorKind::ConnectionAborted {
                    break;
                }
                error!("{:?}", err);
                return Err(CabotError::IOError(format!("{}", err)));
            } else {
                let clearcount = clearcount.unwrap();
                debug!("Read {} clear bytes", clearcount);
                if clearcount > 0 {
                    let spart = unsafe {
                        String::from_utf8_unchecked(part)
                    };
                    response.push_str(spart.as_str());
                }
            }
        } else {
            break;
        }
    }
    write_response(out, response, verbose);
    Ok(())
}


pub fn http_query(request: &Request,
                  mut out: &mut Write,
                  verbose: bool)
                  -> CabotResult<()> {
    debug!("HTTP Query {} {}", request.http_method(), request.request_uri());

    let resolver = Resolver::new();
    let authority = request.authority();
    info!("DNS Lookup {}", authority);
    let addr = resolver.get_addr(authority);
    debug!("Host {} has been resolved to {}", authority, addr);

    info!("Connecting to {}", addr);
    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    let response = match request.scheme() {
        "http" => from_http(request, &mut client, &mut out, verbose)?,
        "https" => from_https(request, &mut client, &mut out, verbose)?,
        _ => {
            return Err(CabotError::SchemeError(format!("Unrecognized scheme {}", request.scheme())))
        }
    };

    out.flush().unwrap();

    Ok(())

}
