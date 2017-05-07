#[macro_use]
extern crate log;

extern crate pretty_env_logger;

extern crate clap;
extern crate cabot;

use std::io;
use std::io::Write;
use std::fs::File;

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
        .get_matches();

    let url = matches.value_of("URL").unwrap();
    let http_method = matches.value_of("REQUEST").unwrap();
    let verbose = matches.is_present("VERBOSE");

    let headers: Vec<&str> = match matches.values_of("LINE") {
        Some(headers) => headers.collect(),
        None => Vec::new(),
    };

    let request = RequestBuilder::new(url).set_http_method(http_method)
        .add_headers(&headers.as_slice())
        .build()?;

    if let Some(path) = matches.value_of("FILE") {
        let mut f = File::create(path).unwrap();
        http::http_query(&request, &mut f, verbose)?;
    } else {
        http::http_query(&request, &mut io::stdout(), verbose)?;
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
            let _ = writeln!(&mut std::io::stderr(), "Opaque URL Error:{}", err);
            std::process::exit(1);
        }
        Err(CabotError::UrlParseError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "URL Parse Error:{}", err);
            std::process::exit(1);
        }
        Err(CabotError::IOError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "IOError:{}", err);
            std::process::exit(1);
        }
        Err(CabotError::CertificateError(err)) => {
            let _ = writeln!(&mut std::io::stderr(), "CertificateError: {}", err);
            std::process::exit(1);
        }        
    }
}
