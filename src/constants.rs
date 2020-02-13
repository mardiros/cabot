//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

use regex::bytes::{Regex as BytesRegex, RegexBuilder as BytesRegexBuilder};
use regex::Regex;

/// Version of cabot
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NUMBER_OF_REDIRECT: u8 = 16;

pub const DNS_LOOKUP_TIMEOUT: u64 = 5;
pub const CONNECT_TIMEOUT: u64 = 15;
pub const READ_TIMEOUT: u64 = 10;
pub const REQUEST_TIMEOUT: u64 = 0;

pub fn user_agent() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Size of the buffer while reading http socket
pub const BUFFER_PAGE_SIZE: usize = 2048;

lazy_static! {
    pub static ref SPLIT_HEADERS_RE: BytesRegex = BytesRegex::new(r"\r\n\r\n").unwrap();
    pub static ref SPLIT_HEADER_BRE: BytesRegex = BytesRegex::new(r"\r\n").unwrap();
    pub static ref GET_CHUNK_SIZE: BytesRegex = BytesRegex::new(r"([0-9A-Fa-f]+)").unwrap();
    pub static ref SPLIT_HEADER_RE: Regex = Regex::new(r"\r\n").unwrap();
    pub static ref TRANSFER_ENCODING: BytesRegex =
        BytesRegexBuilder::new(r"\nTransfer-Encoding:\s*(\S*)")
            .case_insensitive(true)
            .build()
            .expect("Invalid TRANSFER_ENCODING Regex");
    pub static ref CONTENT_LENGTH: BytesRegex =
        BytesRegexBuilder::new(r"\nContent-Length:\s*(\S*)")
            .case_insensitive(true)
            .build()
            .expect("Invalid CONTENT_LENGTH Regex");
    pub static ref LOCATION: BytesRegex = BytesRegexBuilder::new(r"\nLocation:\s*(\S*)")
        .case_insensitive(true)
        .build()
        .expect("Invalid LOCATION Regex");
}
