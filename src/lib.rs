#[macro_use]
extern crate log;

extern crate url;
extern crate rustls;
extern crate webpki_roots;

mod dns;
pub mod http;

pub mod results;
pub mod request;
pub mod executor;
mod response;
