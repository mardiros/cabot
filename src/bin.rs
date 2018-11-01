#[macro_use]
extern crate log;

extern crate pretty_env_logger;

extern crate cabot;
extern crate clap;

use std::collections::HashMap;
use std::fmt::Arguments;
use std::fs::OpenOptions;
use std::io::{self, stderr, Write};
use std::iter::FromIterator;
use std::net::{AddrParseError, SocketAddr};

use log::Level::Info;

use clap::{App, Arg};

use cabot::constants;
use cabot::http;
use cabot::request::RequestBuilder;
use cabot::results::CabotResult;

pub fn run() -> CabotResult<()> {
    let matches = App::new("cabot")
        .version(constants::VERSION)
        .author("Guillaume Gauvrit <guillaume@gauvr.it>")
        .about("http(s) client")
        .arg(
            Arg::with_name("URL")
                .index(1)
                .required(true)
                .help("URL to request"),
        ).arg(
            Arg::with_name("REQUEST")
                .short("X")
                .long("request")
                .default_value("GET")
                .help("Specify request command to use"),
        ).arg(
            Arg::with_name("HEADER")
                .short("H")
                .long("header")
                .takes_value(true)
                .multiple(true)
                .help("Pass custom header to server"),
        ).arg(
            Arg::with_name("FILE")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Write to FILE instead of stdout"),
        ).arg(
            Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .help("Make the operation more talkative"),
        ).arg(
            Arg::with_name("BODY")
                .short("d")
                .long("data")
                .takes_value(true)
                .help("Post Data (Using utf-8 encoding)"),
        ).arg(
            Arg::with_name("IPv4")
                .short("4")
                .long("ipv4")
                .help("Resolve host names to IPv4 addresses"),
        ).arg(
            Arg::with_name("IPv6")
                .short("6")
                .long("ipv6")
                .help("Resolve host names to IPv6 addresses"),
        ).arg(
            Arg::with_name("UA")
                .short("A")
                .long("user-agent")
                .default_value(constants::USER_AGENT)
                .help("Post Data (Using utf-8 encoding)"),
        ).arg(
            Arg::with_name("RESOLVE")
                .long("resolve")
                .takes_value(true)
                .multiple(true)
                .help("<host:port:address> Resolve the host+port to this address"),
        ).get_matches();

    let url = matches.value_of("URL").unwrap();
    let http_method = matches.value_of("REQUEST").unwrap();
    let verbose = matches.is_present("VERBOSE");
    let body = matches.value_of("BODY");
    let ua = matches.value_of("UA").unwrap();

    let mut ipv4 = matches.is_present("IPv4");
    let mut ipv6 = matches.is_present("IPv6");
    if !ipv4 && !ipv6 {
        ipv4 = true;
        ipv6 = true;
    }

    let headers: Vec<&str> = match matches.values_of("HEADER") {
        Some(headers) => headers.collect(),
        None => Vec::new(),
    };

    let resolved: HashMap<String, SocketAddr> = match matches.values_of("RESOLVE") {
        Some(headers) => HashMap::from_iter(
            headers
                .map(|resolv| resolv.splitn(3, ':'))
                .map(|resolv| {
                    let count = resolv.clone().count();
                    if count != 3 {
                        let resolv: Vec<&str> = resolv.collect();
                        let _ = writeln!(
                            &mut std::io::stderr(),
                            "Invalid format in resolve argument: {}",
                            resolv.join(":")
                        );
                        std::process::exit(1);
                    }
                    resolv
                }).map(|mut resolv| {
                    (
                        resolv.next().unwrap(),
                        resolv.next().unwrap(),
                        resolv.next().unwrap(),
                    )
                }).map(|(host, port, addr)| {
                    let parsed_port = port.parse::<usize>();
                    if parsed_port.is_err() {
                        let _ = writeln!(
                            &mut std::io::stderr(),
                            "Invalid port in resolve argument: {}:{}:{}",
                            host,
                            port,
                            addr
                        );
                        std::process::exit(1);
                    }
                    let sockaddr: Result<SocketAddr, AddrParseError> =
                        format!("{}:{}", addr, port).parse();
                    if sockaddr.is_err() {
                        let _ = writeln!(
                            &mut std::io::stderr(),
                            "Invalid address in resolve argument: {}:{}:{}",
                            host,
                            port,
                            addr
                        );
                        std::process::exit(1);
                    }
                    let sockaddr = sockaddr.unwrap();
                    (format!("{}:{}", host, port), sockaddr)
                }),
        ),
        None => HashMap::new(),
    };

    let mut builder = RequestBuilder::new(url)
        .set_http_method(http_method)
        .set_user_agent(ua)
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
            .open(path)
            .unwrap();
        http::http_query(
            &request,
            &mut CabotBinWrite::new(&mut f, verbose),
            &resolved,
            verbose,
            ipv4,
            ipv6,
        )?;
    } else {
        http::http_query(
            &request,
            &mut CabotBinWrite::new(&mut io::stdout(), verbose),
            &resolved,
            verbose,
            ipv4,
            ipv6,
        )?;
    };

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    debug!("Starting cabot");
    match run() {
        Ok(()) => {
            debug!("Command cabot ended succesfully");
        }
        Err(err) => {
            let _ = writeln!(&mut std::io::stderr(), "{}", err);
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
        CabotBinWrite { out, verbose }
    }
}

impl<'a> Write for CabotBinWrite<'a> {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let response: Vec<&[u8]> = constants::SPLIT_HEADERS_RE.splitn(buf, 2).collect();

        // If there is headers and we logged them
        if response.len() == 2 && (log_enabled!(Info) || self.verbose) {
            let headers = &response[0];
            let headers = String::from_utf8_lossy(headers);
            let split: Vec<&str> = constants::SPLIT_HEADER_RE.split(&headers).collect();
            if log_enabled!(Info) {
                for part in split {
                    info!("< {}", part);
                }
            } else if self.verbose {
                for part in split {
                    writeln!(&mut stderr(), "< {}", part).unwrap();
                }
            }
        }

        let body = if response.len() == 2 {
            let start = &response[0].len() + 4;
            &buf[start..]
        } else {
            &buf[..]
        };

        if log_enabled!(Info) {
            info!("< [[{} bytes]]", body.len());
        } else if self.verbose {
            writeln!(&mut stderr(), "< [[{} bytes]]", body.len()).unwrap();
        }

        self.out.write_all(body)?;
        self.flush()?;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }

    // Don't implemented unused method

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }

    fn write_fmt(&mut self, _: Arguments) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "Not Implemented"))
    }
}
