//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

use regex::bytes::{Regex as BytesRegex, RegexBuilder as BytesRegexBuilder};
use regex::Regex;

/// Version of cabot
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn user_agent() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

lazy_static! {
    pub static ref SPLIT_HEADERS_RE: BytesRegex = BytesRegex::new(r"\r\n\r\n").unwrap();
    pub static ref SPLIT_HEADER_BRE: BytesRegex = BytesRegex::new(r"\r\n").unwrap();
    pub static ref GET_CHUNK_SIZE: BytesRegex = BytesRegex::new(r"([0-9A-Fa-f]+)").unwrap();
    pub static ref SPLIT_HEADER_RE: Regex = Regex::new(r"\r?\n").unwrap();
    pub static ref TRANSFER_ENCODING: BytesRegex =
        BytesRegexBuilder::new(r"\nTransfer-Encoding:\s*(\S*)")
            .case_insensitive(true)
            .build()
            .expect("Invalid TRANSFER_ENCODING Regex");
}
