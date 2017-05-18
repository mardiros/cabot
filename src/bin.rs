#[macro_use]
extern crate log;

extern crate pretty_env_logger;

extern crate clap;
extern crate cabot;

use std::fmt::Arguments;
use std::fs::OpenOptions;
use std::io::{self, Write, stderr};

use log::LogLevel::Info;

use clap::{App, Arg};

use cabot::results::{CabotResult, CabotError};
use cabot::http;
use cabot::request::RequestBuilder;

const VERSION: &'static str = "0.1.0";


pub fn run() -> CabotResult<()> {
    let matches = App::new("cabot")
        .version(VERSION)
        .author("Guillaume Gauvrit <guillaume@gauvr.it>")
        .about("http(s) client")
        .arg(Arg::with_name("URL")
            .index(1)
            .required(true)
            .help("URL to request"))
        .arg(Arg::with_name("REQUEST")
            .short("X")
            .long("request")
            .default_value("GET")
            .help("Specify request command to use"))
        .arg(Arg::with_name("LINE")
            .short("H")
            .long("header")
            .takes_value(true)
            .multiple(true)
            .help("Pass custom header LINE to server"))
        .arg(Arg::with_name("FILE")
            .short("o")
            .long("output")
            .takes_value(true)
            .help("Write to FILE instead of stdout"))
        .arg(Arg::with_name("VERBOSE")
            .short("v")
            .long("verbose")
            .help("Make the operation more talkative"))
        .arg(Arg::with_name("BODY")
            .short("d")
            .long("data")
            .takes_value(true)
            .help("Post Data (Using utf-8 encoding)"))

        .get_matches();

    let url = matches.value_of("URL").unwrap();
    let http_method = matches.value_of("REQUEST").unwrap();
    let verbose = matches.is_present("VERBOSE");
    let body = matches.value_of("BODY");

    let headers: Vec<&str> = match matches.values_of("LINE") {
        Some(headers) => headers.collect(),
        None => Vec::new(),
    };

    let mut builder = RequestBuilder::new(url)
        .set_http_method(http_method)
        .add_headers(&headers.as_slice());

    if body.is_some() {
        builder = builder.set_body_as_str(body.unwrap());
    }

    let request = builder.build()?;

    if let Some(path) = matches.value_of("FILE") {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path).unwrap();
        http::http_query(&request, &mut CabotBinWrite::new(&mut f, verbose), verbose)?;
    } else {
        http::http_query(&request, &mut CabotBinWrite::new(&mut io::stdout(), verbose), verbose)?;
    };

    Ok(())
}



fn main() {
    pretty_env_logger::init().unwrap();
    debug!("Starting cabot");
    match run() {
        Ok(()) => {
            debug!("Command cabot ended succesfully");
        }
        Err(CabotError::SchemeError(scheme)) => {
            let _ = writeln!(&mut std::io::stderr(), "Unamanaged scheme: {}", scheme);
            std::process::exit(1);
        }
        Err(CabotError::OpaqueUrlError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "Opaque URL Error: {}", err);
            std::process::exit(1);
        }
        Err(CabotError::UrlParseError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "URL Parse Error: {}", err);
            std::process::exit(1);
        }
        Err(CabotError::IOError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "IOError: {}", err);
            std::process::exit(1);
        }
        Err(CabotError::CertificateError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "CertificateError: {}", err);
            std::process::exit(1);
        }        
        // Unexpexcted Error, not used
        Err(CabotError::HttpResponseParseError(_)) => {
            let _ = writeln!(&mut std::io::stderr(), "Unexpected error");
            std::process::exit(1);
        }
        Err(CabotError::EncodingError(_)) => {
            let _ = writeln!(&mut std::io::stderr(), "Unexpected error");
            std::process::exit(1);
        }
    }
}


// Internal Of the Binary


struct CabotBinWrite<'a> {
    out: &'a mut Write,
    verbose: bool,
}

impl<'a> CabotBinWrite<'a> {
    pub fn new(out: &'a mut Write, verbose: bool) -> Self {
        CabotBinWrite{ out: out, verbose: verbose}
    }
}


impl<'a> Write for CabotBinWrite<'a> {

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {


        let response = String::from_utf8_lossy(buf);
        let response: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();

        // If there is headers and we logged them
        if response.len() == 2 && (log_enabled!(Info) || self.verbose) {
            let headers = response.get(0).unwrap();
            let split = headers.split("\n");
            if log_enabled!(Info) {
                for part in split {
                    info!("< {}", part);
                }
            }
            else if self.verbose {
                for part in split {
                    writeln!(&mut stderr(), "< {}", part).unwrap();
                }
            }
        }

        let body = if response.len() == 2 {
            response.get(1).unwrap()
        } else {
            response.get(0).unwrap()
        };

        if log_enabled!(Info) {
            info!("< [[{} bytes]]", body.len());
        }
        else if self.verbose {
            writeln!(&mut stderr(), "< [[{} bytes]]", body.len()).unwrap();
        }

        self.out.write_all(body.as_bytes()).unwrap();
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        stderr().flush()
    }

    // Don't implemented unused method

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }


    fn write_fmt(&mut self, _: Arguments) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }

}
