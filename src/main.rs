#[macro_use]
extern crate log;
extern crate pretty_env_logger;

extern crate clap;
extern crate url;
extern crate rustls;
extern crate webpki_roots;

use std::io::Write;

mod command;
mod results;
mod dns;
mod http;
mod request;

fn main() {
    pretty_env_logger::init().unwrap();
    debug!("Starting cabot");
    match command::run() {
        Ok(()) => {
            debug!("Command cabot ended succesfully");
        }
        Err(results::CabotError::SchemeError(scheme)) => {
            let _ = writeln!(&mut std::io::stderr(), "Unamanaged scheme: {}", scheme);
            std::process::exit(1);
        }
        Err(results::CabotError::OpaqueUrlError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "Opaque URL Error:{}", err);
            std::process::exit(1);
        }
        Err(results::CabotError::UrlParseError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "URL Parse Error:{}", err);
            std::process::exit(1);
        }
        Err(results::CabotError::IOError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "IOError:{}", err);
            std::process::exit(1);
        }
        Err(results::CabotError::CertificateError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "CertificateError: {}", err);
            std::process::exit(1);
        }        
    }
}
