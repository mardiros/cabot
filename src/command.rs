use std::io;
use std::io::Write;
use std::io::prelude::*;
use std::fs::File;

use clap::{App, Arg};
use url::Url;

use super::results::{CabotResult, CabotError};
use super::http;
use super::request::RequestBuilder;

pub fn run() -> CabotResult<()> {
    let matches = App::new("cabot")
        .version("0.1.0")
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
            .multiple(true)
            .help("Write to FILE instead of stdout"))
        .get_matches();

    let url = matches.value_of("URL").unwrap();
    let http_method = matches.value_of("REQUEST").unwrap();

    let headers: Vec<&str> = match matches.values_of("LINE") {
        Some(headers) => headers.collect(),
        None =>  Vec::new(),
    };

    let request = RequestBuilder::new(url)
        .set_http_method(http_method)
        .add_headers(&headers.as_slice())
        .build()?;

    if let Some(path) = matches.value_of("FILE") {
        let mut f = File::create(path).unwrap();
        http::http_query(&request, &mut f)?;
    }
    else {
        http::http_query(&request, &mut io::stdout())?;
    };

    Ok(())
}
