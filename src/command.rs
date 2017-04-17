use clap::{App, Arg};
use url::Url;

use super::results::{CabotResult, CabotError};
use super::http;
use super::request::RequestBuilder;

pub fn run() -> CabotResult<()> {
    let matches = App::new("cabot")
        .version("0.1.0")
        .author("Guillaume Gauvrit <guillaume@gauvr.it>")
<<<<<<< HEAD
        .about("client URL request library riir")
=======
        .about("http(s) client")
>>>>>>> 315dc97... fixup! Add minimal working version
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
        .arg(Arg::with_name("URL")
            .index(1)
            .required(true)
            .help("URL to request"))
        .get_matches();

    let url = matches.value_of("URL").unwrap();
    let http_method = matches.value_of("REQUEST").unwrap();
    let headers: Vec<&str> = matches.values_of("LINE").unwrap().collect();

    let request =  RequestBuilder::new(url)
        .set_http_method(http_method)
        .add_headers(&headers.as_slice())
        .build()?;

    let response = http::http_query(&request)?;
    println!("{}", String::from_utf8_lossy(response.as_slice()));
    Ok(())
}
