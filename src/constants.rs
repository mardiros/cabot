//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

use regex::bytes::Regex as BytesRegex;
use regex::Regex;

/// Version of cabot
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn user_agent() -> String {
  format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}


lazy_static! {
    pub static ref SPLIT_HEADERS_RE: BytesRegex = BytesRegex::new("\r?\n\r?\n").unwrap();
    pub static ref SPLIT_HEADER_RE: Regex = Regex::new("\r?\n").unwrap();
}
