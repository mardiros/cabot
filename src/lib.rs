//! # cabot
//!
//! cabot is a command line tool and a rust library for sending HTTP query.
//! It is a simple implementation of HTTP based on the rust standard library
//! to perform TCP and DNS query, and use rustls for handling HTTPS connection.
//! No tiers library are used for HTTP protocol.
//!
//! cabot is inspired by the cURL command line tool, but focus on the
//! http protocol which is the referent in HTTP client library.
//!
//! # Examples:
//!
//! ## Command Line:
//!
//! ```bash
//! $ cabot https://www.rust-lang.org/en-US/ | head -n 10 | grep "description"
//! <meta name="description" content="A systems programming language that runs 
//! blazingly fast, prevents segfaults, and guarantees thread safety.">
//! ```
//!
//! ## Library:
//!
//! ```
//! use cabot::{RequestBuilder, RequestExecutor};
//!
//! let request = RequestBuilder::new("https://www.rust-lang.org/en-US/")
//!     .build()
//!     .unwrap();
//! let client = RequestExecutor::new();
//! let response = client.execute(&request).unwrap();
//! assert!(response.body_as_string().unwrap().contains("runs blazingly fast"));
//!
//! ```
//!
//! # Why cabot ?
//!
//! To get a simple rust native https client. No binding to OpenSSL.
//!
//! # License
//!
//! BSD 3-Clause License
//!

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
pub mod response;

// Rexport
pub use request::RequestBuilder;
pub use executor::RequestExecutor;
