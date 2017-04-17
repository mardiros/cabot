use std::sync::Arc;
use std::time::Duration;
use std::io::{self, Write};
use std::io::prelude::*;
use std::net::{TcpStream, ToSocketAddrs};

use rustls::{Session, ClientConfig, ClientSession};
use webpki_roots;
use url::Url;

use super::request::Request;
use super::results::{CabotResult, CabotError};
use super::dns::Resolver;

pub fn from_http(request: &Request, mut client: &mut TcpStream, mut out: &mut Write) -> CabotResult<()> {

    let mut response: Vec<u8> = Vec::new();
    let request_str = request.to_string();

    debug!("Sending request {}", request_str);
    client.write(request_str.as_bytes()).unwrap();
    client.read_to_end(&mut response).unwrap();
    out.write_all(response.as_slice());
    Ok(())
}

pub fn from_https(request: &Request, mut client: &mut TcpStream, mut out: &mut Write) -> CabotResult<()> {

    let request_str = request.to_string();

    let mut config = ClientConfig::new();
    config.root_store.add_trust_anchors(&webpki_roots::ROOTS);
    let rc_config = Arc::new(config);
    let mut tlsclient = ClientSession::new(&rc_config, request.host());
    tlsclient.write_all(request_str.as_bytes()).unwrap();

    loop {
        while tlsclient.wants_write() {
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
                out.write_all(&part.as_slice());
                if err.kind() == io::ErrorKind::ConnectionAborted {
                    break;
                }
                error!("{:?}", err);
                return Err(CabotError::IOError(format!("{}", err)));
            } else {
                let clearcount = clearcount.unwrap();
                debug!("Read {} clear bytes", clearcount);
                if clearcount > 0 {
                    out.write_all(&part.as_slice());
                }
            }
        } else {
            break;
        }
    }
    Ok(())
}


pub fn http_query(request: &Request, mut out: &mut Write) -> CabotResult<()> {
    debug!("{} {}", request.http_method(), request.request_uri());

    let resolver = Resolver::new();
    debug!("DNS Lookup: {}", request.authority());
    let addr = resolver.get_addr(request.authority());
    debug!("Addr: {}", addr);

    debug!("Connecting {}", addr);
    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    let response = match request.scheme() {
        "http" => from_http(request, &mut client, &mut out)?,
        "https" => from_https(request, &mut client, &mut out)?,
        _ => {
            return Err(CabotError::SchemeError(format!("Unrecognized scheme {}", request.scheme())))
        }
    };

    out.flush().unwrap();

    Ok(())

}
