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
//! ## Examples:
//!
//! ### Command Line:
//!
//! ```bash
//! $ cargo run -- https://www.rust-lang.org/ 2>&1| head -n 20 | grep 'name="description"'
//!     <meta name="description" content="A language empowering everyone to build reliable and efficient software.">
//! ```
//! ### Library:
//!
//! ```
//! use async_std::task;
//! use cabot::{RequestBuilder, Client};
//!
//! let request = RequestBuilder::new("https://www.rust-lang.org/")
//!     .build()
//!     .unwrap();
//! let client = Client::new();
//! let response = task::block_on(async {client.execute(&request).await.unwrap()});
//! assert!(response.body_as_string().unwrap().contains("Rust is blazingly fast and memory-efficient"));
//!
//! ```
//!
//! ## Features
//! 
//! There is no default features set.
//! 
//!  * `pretty_log`: add pretty_env_logger dependency.
//! 
//! If set, pretty_env_logger is initalize for the CLI command,
//! it has no effect as using cabot as a library.
//! 
//! Usage:
//! 
//! RUST_LOG=cabot cargo run --features=pretty_log  -- <url>
//! 
//! 
//!  * `json`: add `serde_json` dependency.
//! 
//! If set, the [RequestBuilder](request/struct.RequestBuilder.html) struct 
//! as a method `set_body_as_json()`, in order to
//! serialize a struct that implement serde `Serialize` using `serde_json`,
//! and the [Response](response/struct.Response.html) structure a method `json()`
//! that deserialize a json to an object implementing serde Deserialize 
//! using serde_json.
//!
//! ## Why cabot ?
//!
//! To get a simple rust native https client. No binding to OpenSSL.
//!
//! # License
//!
//! BSD 3-Clause License
//!

#[macro_use]
extern crate log;

extern crate rustls;
extern crate url;
extern crate webpki;
extern crate webpki_roots;

mod dns;

mod asynctls;

pub mod client;
pub mod constants;
pub mod errors;
pub mod http;
pub mod request;
pub mod response;
pub mod results;

// Rexport
pub use client::Client;
pub use request::RequestBuilder;
