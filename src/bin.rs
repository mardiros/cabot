#[macro_use]
extern crate log;

use cabot;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::net::{AddrParseError, SocketAddr};
use std::pin::Pin;

use async_std;
use async_std::fs::{File, OpenOptions};
use async_std::io::{self, Stdout, Write};
use async_std::prelude::*;
use async_std::task::Context;
use async_std::task::Poll;
use clap::{App, Arg};
use log::Level::Info;

use cabot::constants;
use cabot::http;
use cabot::request::RequestBuilder;
use cabot::results::CabotResult;

macro_rules! parse_int {
    ($name:expr, $typ:ty, $matches:ident) => {
        <$typ>::from_str_radix($matches.value_of($name).unwrap(), 10)
            .expect(format!("{} must be an integer", $name).as_str())
    };
}

pub async fn run() -> CabotResult<()> {
    let user_agent: String = constants::user_agent();

    let dns_lookup_timeout = format!("{}", constants::DNS_LOOKUP_TIMEOUT);
    let connect_timeout = format!("{}", constants::CONNECT_TIMEOUT);
    let request_timeout = format!("{}", constants::REQUEST_TIMEOUT);
    let read_timeout = format!("{}", constants::READ_TIMEOUT);

    let number_of_redirect = format!("{}", constants::NUMBER_OF_REDIRECT);

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(constants::VERSION)
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("URL")
                .index(1)
                .required(true)
                .help("URL to request"),
        )
        .arg(
            Arg::with_name("REQUEST")
                .short("X")
                .long("request")
                .default_value("GET")
                .help("Specify request command to use"),
        )
        .arg(
            Arg::with_name("HEADER")
                .short("H")
                .long("header")
                .takes_value(true)
                .multiple(true)
                .help("Pass custom header to server"),
        )
        .arg(
            Arg::with_name("FILE")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Write to FILE instead of stdout"),
        )
        .arg(
            Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .help("Make the operation more talkative"),
        )
        .arg(
            Arg::with_name("BODY")
                .short("d")
                .long("data")
                .takes_value(true)
                .help("Post Data (Using utf-8 encoding)"),
        )
        .arg(
            Arg::with_name("IPv4")
                .short("4")
                .long("ipv4")
                .help("Resolve host names to IPv4 addresses"),
        )
        .arg(
            Arg::with_name("IPv6")
                .short("6")
                .long("ipv6")
                .help("Resolve host names to IPv6 addresses"),
        )
        .arg(
            Arg::with_name("UA")
                .short("A")
                .long("user-agent")
                .default_value(user_agent.as_str())
                .help("The user-agent HTTP header to use"),
        )
        .arg(
            Arg::with_name("DNS_LOOKUP_TIMEOUT")
                .long("dns-timeout")
                .takes_value(true)
                .default_value(dns_lookup_timeout.as_str())
                .help("timeout for the dns lookup resolution in seconds"),
        )
        .arg(
            Arg::with_name("CONNECT_TIMEOUT")
                .long("connect-timeout")
                .takes_value(true)
                .default_value(connect_timeout.as_str())
                .help("timeout for the tcp connection"),
        )
        .arg(
            Arg::with_name("READ_TIMEOUT")
                .long("read-timeout")
                .takes_value(true)
                .default_value(read_timeout.as_str())
                .help("timeout for the tcp read in seconds"),
        )
        .arg(
            Arg::with_name("REQUEST_TIMEOUT")
                .long("max-time")
                .takes_value(true)
                .default_value(request_timeout.as_str())
                .help("timeout for the whole http request in seconds (0 means no timeout)"),
        )
        .arg(
            Arg::with_name("NUMBER_OF_REDIRECT")
                .long("max-redirs")
                .takes_value(true)
                .default_value(number_of_redirect.as_str())
                .help("max number of redirection before returning a response"),
        )
        .arg(
            Arg::with_name("RESOLVE")
                .long("resolve")
                .takes_value(true)
                .multiple(true)
                .help("<host:port:address> Resolve the host+port to this address"),
        )
        .get_matches();

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
                        let _ =
                            eprintln!("Invalid format in resolve argument: {}", resolv.join(":"));
                        std::process::exit(1);
                    }
                    resolv
                })
                .map(|mut resolv| {
                    (
                        resolv.next().unwrap(),
                        resolv.next().unwrap(),
                        resolv.next().unwrap(),
                    )
                })
                .map(|(host, port, addr)| {
                    let parsed_port = port.parse::<usize>();
                    if parsed_port.is_err() {
                        let _ = eprintln!(
                            "Invalid port in resolve argument: {}:{}:{}",
                            host, port, addr
                        );
                        std::process::exit(1);
                    }
                    let sockaddr: Result<SocketAddr, AddrParseError> =
                        format!("{}:{}", addr, port).parse();
                    if sockaddr.is_err() {
                        let _ = eprintln!(
                            "Invalid address in resolve argument: {}:{}:{}",
                            host, port, addr
                        );
                        std::process::exit(1);
                    }
                    let sockaddr = sockaddr.unwrap();
                    (format!("{}:{}", host, port), sockaddr)
                }),
        ),
        None => HashMap::new(),
    };

    let dns_timeout = parse_int!("DNS_LOOKUP_TIMEOUT", u64, matches) * 1_000;
    let connect_timeout = parse_int!("CONNECT_TIMEOUT", u64, matches) * 1_000;
    let read_timeout = parse_int!("READ_TIMEOUT", u64, matches) * 1_000;
    let request_timeout = parse_int!("REQUEST_TIMEOUT", u64, matches) * 1_000;
    let number_of_redirect = parse_int!("NUMBER_OF_REDIRECT", u8, matches);

    let mut builder = RequestBuilder::new(url)
        .set_http_method(http_method)
        .set_user_agent(ua)
        .add_headers(&headers.as_slice());

    if body.is_some() {
        builder = builder.set_body_as_str(body.unwrap());
    }

    let request = builder.build()?;

    let mut file: Option<File>;
    let mut stdout: Option<Stdout>;

    let mut out = if let Some(path) = matches.value_of("FILE") {
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .unwrap();
        file = Some(f);
        CabotBinWrite::new(file.as_mut().unwrap(), verbose)
    } else {
        stdout = Some(io::stdout());
        CabotBinWrite::new(stdout.as_mut().unwrap(), verbose)
    };

    http::http_query(
        &request,
        &mut out,
        &resolved,
        verbose,
        ipv4,
        ipv6,
        dns_timeout,
        connect_timeout,
        read_timeout,
        request_timeout,
        number_of_redirect,
    )
    .await?;
    Ok(())
}

#[async_std::main]
async fn main() {
    #[cfg(feature = "pretty_log")]
    pretty_env_logger::init();
    debug!("Starting cabot");

    run()
        .await
        .map(|ok| {
            debug!("Command cabot ended succesfully");
            ok
        })
        .map_err(|err| {
            eprintln!("{}", err);
            std::process::exit(1);
        })
        .unwrap();
}

// Internal Of the Binary

struct CabotBinWrite<'a> {
    out: &'a mut (dyn Write + Unpin),
    header_read: bool,
    verbose: bool,
}

impl<'a> CabotBinWrite<'a> {
    pub fn new(out: &'a mut (dyn Write + Unpin), verbose: bool) -> Self {
        CabotBinWrite {
            out,
            verbose,
            header_read: false,
        }
    }
    fn display_headers(&self, buf: &[u8]) {
        let headers = String::from_utf8_lossy(buf);
        let split: Vec<&str> = constants::SPLIT_HEADER_RE.split(&headers).collect();
        if log_enabled!(Info) {
            for part in split {
                info!("< {}", part);
            }
        } else if self.verbose {
            for part in split {
                eprintln!("< {}", part);
            }
        }
    }
}

impl<'a> Write for CabotBinWrite<'a> {
    // may receive headers
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let self_ = Pin::get_mut(self);
        if !self_.header_read {
            // the first time write is called, all headers are sent
            // in the buffer. there is no need to parse it again.
            if log_enabled!(Info) || self_.verbose {
                self_.display_headers(&buf);
            }
            self_.header_read = true;
            Poll::Ready(Ok(0))
        } else {
            let towrite = buf.len();
            let mut written = 0;
            loop {
                let res = Pin::new(&mut self_.out).poll_write(cx, &buf[written..towrite]);
                match res {
                    Poll::Ready(Ok(l)) => written += l,
                    _ => {}
                }
                if written >= towrite {
                    break;
                }
            }
            Poll::Ready(Ok(written))
        }
    }

    /// this function is called when the request is done
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        let self_ = Pin::get_mut(self);
        self_.out.flush();
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        let self_ = Pin::get_mut(self);
        Pin::new(&mut self_.out).poll_close(cx)
    }
}
